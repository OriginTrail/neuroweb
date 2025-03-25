use frame_support::traits::{Get, UncheckedOnRuntimeUpgrade};

use sp_runtime::{Perbill, Percent};
use crate::inflation::{Range, InflationInfo};

pub struct InnerMigrateV0ToV1<T: crate::Config>(core::marker::PhantomData<T>);

impl<T: crate::Config> UncheckedOnRuntimeUpgrade for InnerMigrateV0ToV1<T> {

    #[cfg(feature = "try-runtime")]
    fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
        use codec::Encode;

        let comission = crate::CollatorCommision::<T>::get();
        let inflation = crate::InflationConfig::<T>::get();
        let bond_info = crate::ParachainBondInfo::<T>::get();
        let total = crate::TotalSelected::<T>::get();
				
	    Ok((commission, inflation, bond_info, total).encode())
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

        let num_selected_candidates = 1;
        // Must be bigger than MinSelectedCandidates
        crate::TotalSelected::<T>::put(num_selected_candidates);
        // crate::Delegations::<T, Balance>::put(Default::default());

        let mut round = crate::Round::<T>::get();
		let (now, first, old) = (round.current, round.first, round.length);
        round.length = 3600;
        crate::Round::<T>::put(round);
				
        T::DbWeight::get().reads_writes(2, 5)
	}

    #[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		use frame_support::ensure;

        let comission = crate::CollatorCommission::<T>::get();
        ensure!(comission.is_some(), "Commission value is not set");

        let inflation = crate::InflationConfig::<T>::get();
        ensure!(inflation.is_some(), "Inflation value is not set");

        let bond_info = crate::BondInfo::<T>::get();
        ensure!(bond_info.is_some(), "Bond info value is not set");

        let num_selected_candidates = crate::TotalSelected::<T>::get();
        ensure!(num_selected_candidates.is_some(), "Total selected candidates value is not set");

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


