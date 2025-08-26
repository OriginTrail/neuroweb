// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Wrapper;

use frame_benchmarking::{account, benchmarks};
use frame_support::traits::fungibles::{Inspect, Mutate};
use frame_system::RawOrigin;
use primitives::UnifiedAssetId;
use sp_runtime::traits::{StaticLookup, Zero};
use xcm::v4::Location;

pub const NEURO: u128 = 1_000_000_000_000;
pub const TRAC: u128 = 1_000_000_000_000_000;

benchmarks! {
    where_clause { where
        T: Config<AssetId = UnifiedAssetId>,
        T: Config<Balance = u128>,
        T: pallet_assets::Config<AssetIdParameter = codec::Compact<u128>>,
        T: pallet_assets::Config<pallet_assets::Instance2, AssetIdParameter = Location>,
        T: pallet_balances::Config<Balance = u128>,
    }
    trac_wrap {
        let caller: T::AccountId = account("caller", 0, 0);
        let amount = 3 * TRAC;

        setup_assets::<T>(caller.clone())?;

        // Give caller some foreign TRAC to wrap
        <T as Config>::Currency::mint_into(T::ForeignTracAssetId::get(), &caller, amount)?;
    }: _(RawOrigin::Signed(caller.clone()), amount)
    verify {
        // Verify foreign TRAC was transferred to pallet account
        assert_eq!(<T as Config>::Currency::balance(T::ForeignTracAssetId::get(), &Wrapper::<T>::pallet_account_id()), amount);

        // Verify local TRAC was minted to caller
        assert_eq!(<T as Config>::Currency::balance(T::LocalTracAssetId::get(), &caller), amount);
    }

    trac_unwrap {
        let caller: T::AccountId = account("caller", 0, 0);
        let amount = 3 * TRAC;

        setup_assets::<T>(caller.clone())?;

        // Give pallet some foreign TRAC and caller some local TRAC
        <T as Config>::Currency::mint_into(T::ForeignTracAssetId::get(), &Wrapper::<T>::pallet_account_id(), amount)?;
        <T as Config>::Currency::mint_into(T::LocalTracAssetId::get(), &caller, amount)?;
    }: _(RawOrigin::Signed(caller.clone()), amount)
    verify {
        // Verify local TRAC was burned from caller
        assert!(<T as Config>::Currency::balance(T::LocalTracAssetId::get(), &caller).is_zero());

        // Verify foreign TRAC was transferred from pallet to caller
        assert_eq!(<T as Config>::Currency::balance(T::ForeignTracAssetId::get(), &caller), amount);
        assert!(<T as Config>::Currency::balance(T::ForeignTracAssetId::get(), &Wrapper::<T>::pallet_account_id()).is_zero());
    }

    pause {
        // Ensure pallet is not paused initially
        assert!(!Wrapper::<T>::is_paused());
    }: _(RawOrigin::Root)
    verify {
        // Verify pallet is now paused
        assert!(Wrapper::<T>::is_paused());
    }

    unpause {
        // First pause the pallet
        Wrapper::<T>::pause(RawOrigin::Root.into())?;
        assert!(Wrapper::<T>::is_paused());
    }: _(RawOrigin::Root)
    verify {
        // Verify pallet is now unpaused
        assert!(!Wrapper::<T>::is_paused());
    }
}

fn setup_assets<T: Config>(caller: T::AccountId) -> DispatchResult
where
    T: Config<AssetId = UnifiedAssetId>,
    T: pallet_assets::Config<AssetIdParameter = codec::Compact<u128>>,
    T: pallet_assets::Config<pallet_assets::Instance2, AssetIdParameter = Location>,
    T: pallet_balances::Config<Balance = u128>,
{
    let pallet_account = Wrapper::<T>::pallet_account_id();

    // Native asset (needed for tx fees)
    let initial_neuro_balance = 100 * NEURO;
    <pallet_balances::Pallet<T> as frame_support::traits::Currency<T::AccountId>>::make_free_balance_be(&caller, initial_neuro_balance.into());
    <pallet_balances::Pallet<T> as frame_support::traits::Currency<T::AccountId>>::make_free_balance_be(&pallet_account, initial_neuro_balance.into());

    // Local TRAC
    let local_trac_asset_id = match T::LocalTracAssetId::get() {
        UnifiedAssetId::Local(id) => id,
        _ => panic!(),
    };

    pallet_assets::Pallet::<T>::force_create(
        RawOrigin::Root.into(), // Root required for force_create
        codec::Compact(local_trac_asset_id),
        <T as frame_system::Config>::Lookup::unlookup(caller.clone()),
        true,
        1u32.into(),
    )?;

    // Foreign TRAC
    let foreign_trac_asset_location = match T::ForeignTracAssetId::get() {
        UnifiedAssetId::Foreign(loc) => loc,
        _ => panic!(),
    };

    pallet_assets::Pallet::<T, pallet_assets::Instance2>::force_create(
        RawOrigin::Root.into(),
        foreign_trac_asset_location,
        <T as frame_system::Config>::Lookup::unlookup(caller.clone()),
        true,
        1u32.into(),
    )?;

    Ok(())
}
