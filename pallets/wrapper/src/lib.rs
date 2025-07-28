#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::*,
    traits::{
        fungibles::{Inspect, Mutate},
        Get,
    },
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{AccountIdConversion, AtLeast32BitUnsigned, Zero};

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

pub mod weights;
pub use weights::*;


#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type AssetId: Parameter
            + Member
            + Clone
            + MaybeSerializeDeserialize
            + Ord
            + TypeInfo
            + MaxEncodedLen;

        /// The balance type
        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Codec
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + MaxEncodedLen
            + TypeInfo;

        // Multi-currency support for handling TRAC assets
        type Currency: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
            + Mutate<Self::AccountId>;

        /// The asset ID for local TRAC token
        #[pallet::constant]
        type LocalTracAssetId: Get<Self::AssetId>;

        /// The MultiLocation for foreign TRAC token
        #[pallet::constant]
        type ForeignTracAssetId: Get<Self::AssetId>;

        /// The pallet ID for account derivation
        #[pallet::constant]
        type PalletId: Get<frame_support::PalletId>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;

        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// TRAC tokens wrapped successfully
        TracWrapped {
            who: T::AccountId,
            amount: T::Balance,
        },
        /// TRAC tokens unwrapped successfully
        TracUnwrapped {
            who: T::AccountId,
            amount: T::Balance,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient funds for the operation
        InsufficientFunds,
        /// Amount is zero
        ZeroAmount,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Wrap foreign TRAC tokens into local TRAC tokens
        ///
        /// Transfers foreign TRAC from sender to pallet account and mints equal amount of local TRAC to sender
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::trac_wrap())]
        pub fn trac_wrap(
            origin: OriginFor<T>,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

            let pallet_account = Self::pallet_account_id();
            let foreign_trac_asset_id = T::ForeignTracAssetId::get();
            let local_trac_asset_id = T::LocalTracAssetId::get();


            // Transfer foreign TRAC from user to pallet account
            T::Currency::transfer(
                foreign_trac_asset_id,
                &who,
                &pallet_account,
                amount,
                frame_support::traits::tokens::Preservation::Expendable,
            ).map_err(|_| Error::<T>::InsufficientFunds)?;

            // Mint local TRAC to user
            T::Currency::mint_into(local_trac_asset_id, &who, amount)?;

            // Emit event
            Self::deposit_event(Event::TracWrapped { who, amount });

            Ok(())
        }

        /// Unwrap local TRAC tokens back to foreign TRAC tokens
        ///
        /// Burns local TRAC from sender and transfers equal amount of foreign TRAC from pallet account to sender
        #[pallet::call_index(1)]
        #[pallet::weight(T::WeightInfo::trac_unwrap())]
        pub fn trac_unwrap(
            origin: OriginFor<T>,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

            let pallet_account = Self::pallet_account_id();
            let foreign_trac_asset_id = T::ForeignTracAssetId::get();
            let local_trac_asset_id = T::LocalTracAssetId::get();


            // Burn local TRAC from user
            T::Currency::burn_from(
                local_trac_asset_id,
                &who,
                amount,
                frame_support::traits::tokens::Precision::Exact,
                frame_support::traits::tokens::Fortitude::Polite,
            ).map_err(|_| Error::<T>::InsufficientFunds)?;

            // Transfer foreign TRAC from pallet account to user
            T::Currency::transfer(
                foreign_trac_asset_id,
                &pallet_account,
                &who,
                amount,
                frame_support::traits::tokens::Preservation::Expendable,
            ).map_err(|_| Error::<T>::InsufficientFunds)?;

            // Emit event
            Self::deposit_event(Event::TracUnwrapped { who, amount });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get the account ID of the pallet
        pub fn pallet_account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Get the foreign TRAC balance of the pallet
        pub fn pallet_foreign_trac_balance() -> T::Balance {
            T::Currency::balance(T::ForeignTracAssetId::get(), &Self::pallet_account_id())
        }

        /// Get the total supply of local TRAC tokens
        pub fn local_trac_total_supply() -> T::Balance {
            T::Currency::total_issuance(T::LocalTracAssetId::get())
        }
    }
}
