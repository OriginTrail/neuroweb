#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::manual_inspect)]

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

use frame_support::pallet_prelude::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {}

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::storage]
    #[pallet::getter(fn testnet_mode)]
    pub type TestnetMode<T> = StorageValue<_, bool, ValueQuery>;

    impl<T: Config> Pallet<T> {
        /// Tests Helper
        #[cfg(feature = "std")]
        pub fn set_testnet_mode(testnet_mode: bool) {
            TestnetMode::<T>::put(testnet_mode);
        }
    }
}
