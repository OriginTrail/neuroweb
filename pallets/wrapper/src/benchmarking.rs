#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Wrapper;

use frame_benchmarking::{account, benchmarks};
use frame_support::traits::fungibles::{Inspect, Mutate};
use frame_system::RawOrigin;
use sp_runtime::traits::Zero;

benchmarks! {
    trac_wrap {
        let caller: T::AccountId = account("caller", 0, 0);
        let pallet_account = Wrapper::<T>::pallet_account_id();
        let amount = 1000u32.into();
        let foreign_asset_id = T::ForeignTracAssetId::get();
        let local_asset_id = T::LocalTracAssetId::get();

        // Setup: give caller some foreign TRAC to wrap
        T::Currency::mint_into(foreign_asset_id.clone(), &caller, amount)?;
    }: _(RawOrigin::Signed(caller.clone()), amount)
    verify {
        // Verify foreign TRAC was transferred to pallet account
        assert_eq!(T::Currency::balance(foreign_asset_id.clone(), &pallet_account), amount);
        
        // Verify local TRAC was minted to caller
        assert_eq!(T::Currency::balance(local_asset_id, &caller), amount);
    }

    trac_unwrap {
        let caller: T::AccountId = account("caller", 0, 0);
        let pallet_account = Wrapper::<T>::pallet_account_id();
        let amount = 1000u32.into();
        let foreign_asset_id = T::ForeignTracAssetId::get();
        let local_asset_id = T::LocalTracAssetId::get();

        // Setup: give pallet some foreign TRAC and caller some local TRAC
        T::Currency::mint_into(foreign_asset_id.clone(), &pallet_account, amount)?;
        T::Currency::mint_into(local_asset_id.clone(), &caller, amount)?;
    }: _(RawOrigin::Signed(caller.clone()), amount)
    verify {
        // Verify local TRAC was burned from caller
        assert!(T::Currency::balance(local_asset_id, &caller).is_zero());
        
        // Verify foreign TRAC was transferred from pallet to caller
        assert_eq!(T::Currency::balance(foreign_asset_id.clone(), &caller), amount);
        assert!(T::Currency::balance(foreign_asset_id, &pallet_account).is_zero());
    }

}