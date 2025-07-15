// src/lib.rs
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::*,
    traits::fungibles::{Inspect, Mutate, Transfer},
};
use frame_system::pallet_prelude::*;

// Define a unified AssetId enum
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum AssetId {
    Native,
    Local(u32),
    Foreign(u32),
}

// Router struct
pub struct AssetRouter;

// --- Transfer ---
impl<T: Config> Transfer<T::AccountId> for AssetRouter {
    fn transfer(
        asset: AssetId,
        from: &T::AccountId,
        to: &T::AccountId,
        amount: T::Balance,
        keep_alive: bool,
    ) -> DispatchResult {
        match asset {
            AssetId::Native => pallet_balances::Pallet::<T>::transfer(
                frame_system::RawOrigin::Signed(from.clone()).into(),
                to.clone(),
                amount,
            ),
            AssetId::Local(id) => pallet_assets::Pallet::<T, crate::Instance1>::transfer(
                frame_system::RawOrigin::Signed(from.clone()).into(),
                id,
                to.clone(),
                amount,
            ),
            AssetId::Foreign(id) => pallet_assets::Pallet::<T, crate::Instance2>::transfer(
                frame_system::RawOrigin::Signed(from.clone()).into(),
                id,
                to.clone(),
                amount,
            ),
        }
    }
}

// --- Inspect ---
impl<T: Config> Inspect<T::AccountId> for AssetRouter {
    type AssetId = AssetId;
    type Balance = T::Balance;

    fn balance(asset: Self::AssetId, who: &T::AccountId) -> Self::Balance {
        match asset {
            AssetId::Native => pallet_balances::Pallet::<T>::free_balance(who),
            AssetId::Local(id) => pallet_assets::Pallet::<T, crate::Instance1>::balance(id, who),
            AssetId::Foreign(id) => pallet_assets::Pallet::<T, crate::Instance2>::balance(id, who),
        }
    }

    fn total_issuance(asset: Self::AssetId) -> Self::Balance {
        match asset {
            AssetId::Native => pallet_balances::Pallet::<T>::total_issuance(),
            AssetId::Local(id) => pallet_assets::Pallet::<T, crate::Instance1>::total_issuance(id),
            AssetId::Foreign(id) => pallet_assets::Pallet::<T, crate::Instance2>::total_issuance(id),
        }
    }
}

// --- Mutate ---
impl<T: Config> Mutate<T::AccountId> for AssetRouter {
    fn mint_into(asset: Self::AssetId, dest: &T::AccountId, amount: Self::Balance) -> DispatchResult {
        match asset {
            AssetId::Native => pallet_balances::Pallet::<T>::deposit_creating(dest, amount),
            AssetId::Local(id) => pallet_assets::Pallet::<T, crate::Instance1>::mint(
                frame_system::RawOrigin::Root.into(),
                id,
                dest.clone(),
                amount,
            ),
            AssetId::Foreign(id) => pallet_assets::Pallet::<T, crate::Instance2>::mint(
                frame_system::RawOrigin::Root.into(),
                id,
                dest.clone(),
                amount,
            ),
        };
        Ok(())
    }

    fn burn_from(asset: Self::AssetId, dest: &T::AccountId, amount: Self::Balance) -> DispatchResult {
        match asset {
            AssetId::Native => pallet_balances::Pallet::<T>::withdraw(
                dest,
                amount,
                frame_support::traits::WithdrawReasons::TRANSFER,
                frame_support::traits::ExistenceRequirement::AllowDeath,
            ).map(|_| ()),
            AssetId::Local(id) => pallet_assets::Pallet::<T, crate::Instance1>::burn(
                frame_system::RawOrigin::Signed(dest.clone()).into(),
                id,
                amount,
            ),
            AssetId::Foreign(id) => pallet_assets::Pallet::<T, crate::Instance2>::burn(
                frame_system::RawOrigin::Signed(dest.clone()).into(),
                id,
                amount,
            ),
        }
    }
}

// Pallet configuration trait
pub trait Config: frame_system::Config {
    type Balance: Parameter + AtLeast32BitUnsigned + Default + Copy + MaxEncodedLen;
}

// Dummy instance markers for assets pallets
pub enum Instance1 {}
pub enum Instance2 {}
