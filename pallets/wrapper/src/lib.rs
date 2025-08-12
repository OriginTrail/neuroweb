// # Pallet: TRAC Wrapper
//
// ## Overview
// This pallet enables seamless wrapping and unwrapping between **foreign TRAC tokens** (bridged
// from Ethereum via Snowbridge) and a **local TRAC representation** (native asset on this chain).
//
// ## Key Features
// - **Wrap**: Transfer foreign TRAC from the user to the pallet account and mint the same amount of
// local TRAC to the user.
// - **Unwrap**: Burn local TRAC from the user and transfer an equal amount of foreign TRAC from the
// pallet account to the user.
// - **Pause/Unpause**: Governance-controlled circuit breaker to halt all operations in emergencies.
// - **Events**: Emitted on every wrap, unwrap, pause, and unpause action.
// - **Balance Queries**: Helpers for checking pallet’s foreign TRAC balance and total local TRAC supply.
//
// ## Technical Details
// - Uses the `fungibles` `Inspect`/`Mutate` traits for multi-currency support.
// - Works with both native and asset-pallet-based currencies via `AssetId` and `Balance` generics.
// - `LocalTracAssetId` and `ForeignTracAssetId` are runtime constants pointing to the respective assets.
// - The pallet account is derived from a configurable `PalletId`.
// - Governance authority for pausing is defined via the `PauseOrigin` associated type.
//
// ## Storage
// - `IsPaused`: `bool` — whether pallet operations are currently paused.
//
// ## Events
// - `TracWrapped { who, amount }` — User wrapped foreign → local TRAC.
// - `TracUnwrapped { who, amount }` — User unwrapped local → foreign TRAC.
// - `Paused` / `Unpaused` — Pallet operations toggled.
//
// ## Errors
// - `InsufficientFunds` — Not enough balance for the operation.
// - `ZeroAmount` — Amount provided was zero.
// - `Paused` — Attempt to perform an operation while the pallet is paused.
//
// ## Security Considerations
// - Wrap/unwrap operations are **1:1** with no fees or slippage in this pallet.
// - Pausing capability allows governance to freeze operations during exploits or bridge malfunctions.
// - Asset IDs must be correctly configured at runtime to prevent misrouting of tokens.
//
#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
use frame_support::{
    dispatch::DispatchResult,
    pallet_prelude::*,
    traits::{
        fungibles::{Inspect, Mutate},
        Get, EnsureOrigin,
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

        /// Origin that can pause/unpause the pallet
        type PauseOrigin: EnsureOrigin<Self::RuntimeOrigin>;

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
        /// Pallet paused
        Paused,
        /// Pallet unpaused
        Unpaused,
    }

    #[pallet::storage]
    #[pallet::getter(fn is_paused)]
    pub type IsPaused<T> = StorageValue<_, bool, ValueQuery>;

    #[pallet::error]
    pub enum Error<T> {
        /// Insufficient funds for the operation
        InsufficientFunds,
        /// Amount is zero
        ZeroAmount,
        /// Pallet is paused
        Paused,
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

            // Ensure pallet is not paused
            ensure!(!Self::is_paused(), Error::<T>::Paused);

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

            // Ensure pallet is not paused
            ensure!(!Self::is_paused(), Error::<T>::Paused);

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

        /// Pause the pallet, preventing all transactions
        #[pallet::call_index(2)]
        #[pallet::weight(T::WeightInfo::pause())]
        pub fn pause(origin: OriginFor<T>) -> DispatchResult {
            T::PauseOrigin::ensure_origin(origin)?;

            IsPaused::<T>::put(true);
            Self::deposit_event(Event::Paused);

            Ok(())
        }

        /// Unpause the pallet, allowing transactions again
        #[pallet::call_index(3)]
        #[pallet::weight(T::WeightInfo::unpause())]
        pub fn unpause(origin: OriginFor<T>) -> DispatchResult {
            T::PauseOrigin::ensure_origin(origin)?;

            IsPaused::<T>::put(false);
            Self::deposit_event(Event::Unpaused);

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
