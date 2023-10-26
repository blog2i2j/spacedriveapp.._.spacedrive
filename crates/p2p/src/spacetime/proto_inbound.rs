use std::{
	future::Future,
	pin::Pin,
	sync::{atomic::Ordering, Arc, PoisonError},
};

use libp2p::{core::UpgradeInfo, InboundUpgrade, Stream};
use tokio::io::AsyncReadExt;
use tokio_util::compat::FuturesAsyncReadCompatExt;
use tracing::debug;

use crate::{
	spacetime::{BroadcastStream, UnicastStream},
	Manager, ManagerStreamAction2, PeerId, PeerMessageEvent,
};

use super::SpaceTimeProtocolName;

pub struct InboundProtocol {
	pub(crate) peer_id: PeerId,
	pub(crate) manager: Arc<Manager>,
}

impl UpgradeInfo for InboundProtocol {
	type Info = SpaceTimeProtocolName;
	type InfoIter = [Self::Info; 1];

	fn protocol_info(&self) -> Self::InfoIter {
		[SpaceTimeProtocolName(self.manager.application_name.clone())]
	}
}

impl InboundUpgrade<Stream> for InboundProtocol {
	type Output = ManagerStreamAction2;
	type Error = ();
	type Future = Pin<Box<dyn Future<Output = Result<Self::Output, Self::Error>> + Send + 'static>>;

	fn upgrade_inbound(self, io: Stream, _: Self::Info) -> Self::Future {
		let id = self.manager.stream_id.fetch_add(1, Ordering::Relaxed);
		Box::pin(async move {
			debug!(
				"stream({}, {id}): accepting inbound connection",
				self.peer_id
			);

			let mut io = io.compat();
			let discriminator = io.read_u8().await.unwrap(); // TODO: Timeout on this
			match discriminator {
				crate::spacetime::BROADCAST_DISCRIMINATOR => {
					debug!("stream({}, {id}): broadcast stream accepted", self.peer_id);
					Ok(ManagerStreamAction2::Event(
						PeerMessageEvent {
							stream_id: id,
							peer_id: self.peer_id,
							manager: self.manager.clone(),
							stream: BroadcastStream::new(io),
							_priv: (),
						}
						.into(),
					))
				}
				crate::spacetime::UNICAST_DISCRIMINATOR => {
					debug!("stream({}, {id}): unicast stream accepted", self.peer_id);

					let stream =
						UnicastStream::new_inbound(self.manager.identity.clone(), io).await;

					self.manager
						.state
						.write()
						.unwrap_or_else(PoisonError::into_inner)
						.connected
						.insert(self.peer_id.0, stream.remote_identity().clone());

					Ok(ManagerStreamAction2::Event(
						PeerMessageEvent {
							stream_id: id,
							peer_id: self.peer_id,
							manager: self.manager.clone(),
							stream,
							_priv: (),
						}
						.into(),
					))
				}
				_ => todo!(), // TODO: Error handling
			}
		})
	}
}
