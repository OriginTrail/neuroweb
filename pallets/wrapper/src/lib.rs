#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    dispatch::{DispatchResult, DispatchResultWithPostInfo},
    pallet_prelude::*,
    traits::{
        fungibles::{Inspect, Mutate},
        Get,
    },
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::{AccountIdConversion, Zero};
use codec::Codec;
use sp_runtime::traits::AtLeast32BitUnsigned;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Multi-currency support for handling TRAC assets
        type MultiCurrency: Inspect<Self::AccountId, AssetId = Self::AssetId, Balance = Self::Balance>
        + Mutate<Self::AccountId>;

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
        + TypeInfo
        + From<u128>
        + Into<u128>;

        /// The asset ID for local TRAC token
        #[pallet::constant]
        type LocalTracAssetId: Get<Self::AssetId>;

        /// The MultiLocation for foreign TRAC token
        #[pallet::constant]
        type ForeignTracAssetId: Get<Self::AssetId>;

        /// The pallet ID for account derivation
        #[pallet::constant]
        type PalletId: Get<frame_support::PalletId>;
    }

    #[pallet::storage]
    #[pallet::getter(fn something)]
    pub type Something<T> = StorageValue<_, u32>;

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
        /// Insufficient foreign TRAC balance
        InsufficientForeignBalance,
        /// Insufficient local TRAC balance
        InsufficientLocalBalance,
        /// Insufficient pallet foreign TRAC balance for unwrapping
        InsufficientPalletBalance,
        /// Transfer failed
        TransferFailed,
        /// Mint failed
        MintFailed,
        /// Burn failed
        BurnFailed,
        /// Amount is zero
        ZeroAmount,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Wrap foreign TRAC tokens into local TRAC tokens
        ///
        /// Transfers foreign TRAC from sender to pallet account and mints equal amount of local TRAC to sender
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn trac_wrap(
            origin: OriginFor<T>,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

            let pallet_account = Self::pallet_account_id();
            let foreign_trac_asset = T::ForeignTracAssetId::get();
            let local_trac_asset = T::LocalTracAssetId::get();

            // Check if user has sufficient foreign TRAC balance
            let user_foreign_balance = T::MultiCurrency::balance(foreign_trac_asset.clone(), &who);
            ensure!(
                user_foreign_balance >= amount,
                Error::<T>::InsufficientForeignBalance
            );

            // Transfer foreign TRAC from user to pallet account
            T::MultiCurrency::transfer(
                foreign_trac_asset,
                &who,
                &pallet_account,
                amount,
                frame_support::traits::tokens::Preservation::Expendable,
            )
                .map_err(|_| Error::<T>::TransferFailed)?;

            // Mint local TRAC to user
            T::MultiCurrency::mint_into(local_trac_asset, &who, amount)
                .map_err(|_| Error::<T>::MintFailed)?;

            // Emit event
            Self::deposit_event(Event::TracWrapped { who, amount });

            Ok(().into())
        }

        /// Unwrap local TRAC tokens back to foreign TRAC tokens
        ///
        /// Burns local TRAC from sender and transfers equal amount of foreign TRAC from pallet account to sender
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn trac_unwrap(
            origin: OriginFor<T>,
            #[pallet::compact] amount: T::Balance,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // Ensure amount is not zero
            ensure!(!amount.is_zero(), Error::<T>::ZeroAmount);

            let pallet_account = Self::pallet_account_id();
            let foreign_trac_asset = T::ForeignTracAssetId::get();
            let local_trac_asset = T::LocalTracAssetId::get();

            // Check if user has sufficient local TRAC balance
            let user_local_balance = T::MultiCurrency::balance(local_trac_asset.clone(), &who);
            ensure!(
                user_local_balance >= amount,
                Error::<T>::InsufficientLocalBalance
            );

            // Check if pallet has sufficient foreign TRAC balance
            let pallet_foreign_balance = T::MultiCurrency::balance(foreign_trac_asset.clone(), &pallet_account);
            ensure!(
                pallet_foreign_balance >= amount,
                Error::<T>::InsufficientPalletBalance
            );

            // Burn local TRAC from user
            T::MultiCurrency::burn_from(
                local_trac_asset,
                &who,
                amount,
                frame_support::traits::tokens::Precision::Exact,
                frame_support::traits::tokens::Fortitude::Polite,
            )
                .map_err(|_| Error::<T>::BurnFailed)?;

            // Transfer foreign TRAC from pallet account to user
            T::MultiCurrency::transfer(
                foreign_trac_asset,
                &pallet_account,
                &who,
                amount,
                frame_support::traits::tokens::Preservation::Expendable,
            )
                .map_err(|_| Error::<T>::TransferFailed)?;

            // Emit event
            Self::deposit_event(Event::TracUnwrapped { who, amount });

            Ok(().into())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get the account ID of the pallet
        pub fn pallet_account_id() -> T::AccountId {
            T::PalletId::get().into_account_truncating()
        }

        /// Get the foreign TRAC balance of the pallet
        pub fn pallet_foreign_trac_balance() -> T::Balance {
            T::MultiCurrency::balance(T::ForeignTracAssetId::get(), &Self::pallet_account_id())
        }

        /// Get the total supply of local TRAC tokens
        pub fn local_trac_total_supply() -> T::Balance {
            T::MultiCurrency::total_issuance(T::LocalTracAssetId::get())
        }
    }
}
