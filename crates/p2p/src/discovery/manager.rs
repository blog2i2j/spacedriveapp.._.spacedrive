use std::{
	collections::{HashMap, HashSet},
	future::poll_fn,
	net::SocketAddr,
	pin::pin,
	sync::{Arc, PoisonError, RwLock},
	task::Poll,
};

use libp2p::PeerId;
use tokio::sync::{broadcast, mpsc, Notify};
use tracing::trace;

use crate::{spacetunnel::RemoteIdentity, ManagerConfig, Mdns, ServiceEventInternal};

type ServiceName = String;

pub(crate) type ListenAddrs = HashSet<SocketAddr>;
pub(crate) type State = Arc<RwLock<DiscoveryManagerState>>;

/// DiscoveryManager controls all user-defined [Service]'s and connects them with the network through mDNS and other discovery protocols
pub(crate) struct DiscoveryManager {
	pub(crate) state: State,
	pub(crate) listen_addrs: ListenAddrs,
	pub(crate) application_name: &'static str,
	pub(crate) peer_id: PeerId,
	pub(crate) mdns: Option<Mdns>,
	// TODO: Split these off `DiscoveryManagerState` and parse around on their own struct???
	pub(crate) do_broadcast: Arc<Notify>,
	pub(crate) service_shutdown_rx: mpsc::Receiver<String>,
}

impl DiscoveryManager {
	pub(crate) fn new(
		application_name: &'static str,
		peer_id: PeerId,
		config: &ManagerConfig,
		state: State,
		service_shutdown_rx: mpsc::Receiver<String>,
	) -> Result<Self, mdns_sd::Error> {
		let mut mdns = None;
		if config.enabled {
			mdns = Some(Mdns::new(&application_name, peer_id)?);
		}

		let do_broadcast = state
			.read()
			.unwrap_or_else(PoisonError::into_inner)
			.do_broadcast
			.clone();

		Ok(Self {
			state,
			listen_addrs: Default::default(),
			application_name,
			peer_id,
			mdns,
			do_broadcast,
			service_shutdown_rx,
		})
	}

	/// is called on changes to `self.services` to make sure all providers update their records
	pub(crate) fn do_advertisement(&mut self) {
		println!("C");
		trace!("Broadcasting new service records");

		if let Some(mdns) = &mut self.mdns {
			println!("D");
			mdns.do_advertisement(&self.listen_addrs, &self.state);
		}
	}

	pub(crate) async fn poll(&mut self) {
		println!("A");
		let y = self.do_broadcast.clone();
		let mut do_broadcast_notifier = pin!(y.notified());
		do_broadcast_notifier.as_mut().enable();

		tokio::select! {
			 _ = do_broadcast_notifier => {
				println!("B");
				self.do_advertisement()
			},
			service_name = self.service_shutdown_rx.recv() => {
				if let Some(service_name) = service_name {
					let mut state = self.state.write().unwrap_or_else(PoisonError::into_inner);
					state.services.remove(&service_name);
					state.discovered.remove(&service_name);
					state.known.remove(&service_name);
				}
				self.do_advertisement();
			}
			_ = poll_fn(|cx| {
				if let Some(mdns) = &mut self.mdns {
					return mdns.poll(cx, &self.listen_addrs, &self.state);
				}

				Poll::Pending
			}) => {},
		}
	}

	pub(crate) fn shutdown(&self) {
		if let Some(mdns) = &self.mdns {
			mdns.shutdown();
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct DiscoveryManagerState {
	/// A list of services the current node is advertising w/ their metadata
	pub(crate) services: HashMap<
		ServiceName,
		(
			broadcast::Sender<ServiceEventInternal>,
			HashMap<String, String>,
		),
	>,
	/// A map of organically discovered peers
	pub(crate) discovered: HashMap<ServiceName, HashMap<RemoteIdentity, DiscoveredPeerCandidate>>,
	/// A map of peers we know about. These may be connected or not avaiable.
	/// This is designed around the Relay/NAT hole punching service where we need to emit who we wanna discover
	/// Note: this may contain duplicates with `discovered` as they will *not* be removed from here when found
	pub(crate) known: HashMap<ServiceName, HashSet<RemoteIdentity>>,
	/// Used to trigger an rebroadcast. This should be called when mutating this struct.
	/// You are intended to clone out of this instead of locking the whole struct's `RwLock` each time you wanna use it.
	pub(crate) do_broadcast: Arc<Notify>,
	/// Used to trigger the removal of a `Service`. This is used in the `impl Drop for Service`
	/// You are intended to clone out of this instead of locking the whole struct's `RwLock` each time you wanna use it.
	pub(crate) service_shutdown_tx: mpsc::Sender<String>,
}

impl DiscoveryManagerState {
	pub fn new() -> (Arc<RwLock<Self>>, mpsc::Receiver<String>) {
		let (service_shutdown_tx, service_shutdown_rx) = mpsc::channel(10);

		(
			Arc::new(RwLock::new(Self {
				services: Default::default(),
				discovered: Default::default(),
				known: Default::default(),
				do_broadcast: Default::default(),
				service_shutdown_tx,
			})),
			service_shutdown_rx,
		)
	}
}

#[derive(Debug, Clone)]
pub(crate) struct DiscoveredPeerCandidate {
	pub(crate) peer_id: PeerId,
	pub(crate) meta: HashMap<String, String>,
	pub(crate) addresses: Vec<SocketAddr>,
}
