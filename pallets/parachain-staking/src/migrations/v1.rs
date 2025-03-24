use frame_support::{
    storage_alias,
    traits::{Get, UncheckedOnRuntimeUpgrade},
};

use sp_runtime::{Perbill, Percent};
use crate::inflation::{Range, InflationInfo};

mod v0 {
    use super::*;
		use frame_support::pallet_prelude::ValueQuery;
    use crate::CollatorCommission;

	/// V0 type for [`crate::Value`].
	#[storage_alias]
	pub type CollatorCommision<T: crate::Config> = StorageValue<
        crate::Pallet<T>, 
        CollatorCommission<T>,
        ValueQuery
    >;
}

pub struct InnerMigrateV0ToV1<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: crate::Config> UncheckedOnRuntimeUpgrade for InnerMigrateV0ToV1<T> {

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
        use codec::Encode;

        let old_value = v0::CollatorCommision::<T>::get();

        Ok(old_value.encode())
    }

    fn on_runtime_upgrade() -> frame_support::weights::Weight {
        let new = Perbill::from_percent(30);

        crate::CollatorCommission::<T>::put(new);
				crate::InflationConfig::<T>::put(
						InflationInfo {
								expect: Range {
										min: T::Balance::from(999u32), // Convert to Balance
										ideal: T::Balance::from(999u32),
										max: T::Balance::from(999u32),
								},
								round: Range {
										min: Perbill::from_percent(74),
										ideal: Perbill::from_percent(74),
										max: Perbill::from_percent(74),
								},
								annual: Range {
										min: Perbill::from_percent(75),
										ideal: Perbill::from_percent(75),
										max: Perbill::from_percent(75),
								},
						}
				);

				let old_bond_info = crate::ParachainBondInfo::<T>::get();
				crate::ParachainBondInfo::<T>::put(crate::ParachainBondConfig {
						account: old_bond_info.account,
						percent: Percent::from_percent(30)
				});
						
				
        T::DbWeight::get().writes(3)
	}

    #[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		use frame_support::ensure;

		let maybe_old_value = crate::CollatorCommission::decode(&mut &state[..]).map_err(|_| {
			sp_runtime::TryRuntimeError::Other("Failed to decode old value from storage")
		})?;

		match maybe_old_value {
			Some(old_value) => {
				let new_value = crate::CollatorCommission::<T>::get();
				ensure!(new_value.is_some(), "New value not set");
				ensure!(
					new_value == Prebil(from_percent(30)),
					"New value not set correctly"
				);
			},
			None => {
				ensure!(crate::CollatorCommission::<T>::get().is_none(), "New value unexpectedly set");
			}
		};

		Ok(())
	}
}


pub type MigrateV0ToV1<T> = frame_support::migrations::VersionedMigration<
	0, // The migration will only execute when the on-chain storage version is 0
	1, // The on-chain storage version will be set to 1 after the migration is complete
	InnerMigrateV0ToV1<T>,
	crate::pallet::Pallet<T>,
	<T as frame_system::Config>::DbWeight,
>;


