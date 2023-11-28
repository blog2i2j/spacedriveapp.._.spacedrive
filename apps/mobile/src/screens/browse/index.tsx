import { AppLogo } from '@sd/assets/images';
import { CheckCircle, Gear } from 'phosphor-react-native';
import React, { useRef } from 'react';
import { Image, Pressable, ScrollView, Text, View } from 'react-native';
import { JobManagerContextProvider, useLibraryQuery } from '@sd/client';
import { PulseAnimation } from '~/components/animation/lottie';
import BrowseLocations from '~/components/browse/BrowseLocations';
import BrowseTags from '~/components/browse/BrowseTags';
import BrowseLibraryManager from '~/components/browse/DrawerLibraryManager';
import { ModalRef } from '~/components/layout/Modal';
import { JobManagerModal } from '~/components/modal/job/JobManagerModal';
import { tw, twStyle } from '~/lib/tailwind';
import { BrowseStackScreenProps } from '~/navigation/tabs/BrowseStack';

function JobIcon() {
	const { data: isActive } = useLibraryQuery(['jobs.isActive']);
	return isActive ? (
		<PulseAnimation style={tw`h-[24px] w-[32px]`} speed={1.5} />
	) : (
		<CheckCircle color="white" size={24} />
	);
}

export default function BrowseScreen({ navigation, route }: BrowseStackScreenProps<'Browse'>) {
	const modalRef = useRef<ModalRef>(null);

	return (
		<ScrollView style={tw`flex-1 px-3 py-2`}>
			<View style={twStyle('justify-between')}>
				<View>
					<View style={tw`mt-6`} />
					{/* Library Manager */}
					<BrowseLibraryManager />
					{/* Locations */}
					<BrowseLocations />
					{/* Tags */}
					<BrowseTags />
				</View>
				<View style={tw`flex w-full flex-row items-center gap-x-4`}>
					{/* Settings */}
					<Pressable onPress={() => navigation.navigate('Settings', { screen: 'Home' })}>
						<Gear color="white" size={24} />
					</Pressable>
					{/* Job Manager */}
					<JobManagerContextProvider>
						<Pressable onPress={() => modalRef.current?.present()}>
							<JobIcon />
						</Pressable>
						<JobManagerModal ref={modalRef} />
					</JobManagerContextProvider>
				</View>
			</View>
		</ScrollView>
	);
}
