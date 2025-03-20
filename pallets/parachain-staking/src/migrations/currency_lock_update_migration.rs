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

//! # Migrations
//!
#[allow(unused_imports)]
use crate::*;

// Substrate
use frame_support::traits::{
	fungible::{InspectHold, MutateHold},
	Currency, Get, LockIdentifier, LockableCurrency, ReservableCurrency,
};
#[allow(unused_imports)]
use frame_support::{migration, storage::unhashed};
use log;
use codec::Encode;
use sp_core::hexdisplay::HexDisplay;
#[cfg(feature = "try-runtime")]
use sp_runtime::DispatchError;
#[allow(unused_imports)]
use sp_std::vec::Vec;

// Lock Identifiers used in the old version of the pallet.
const COLLATOR_LOCK_ID: LockIdentifier = *b"stkngcol";
const DELEGATOR_LOCK_ID: LockIdentifier = *b"stkngdel";

pub struct CustomOnRuntimeUpgrade<T, OldCurrency>
where
	T: Config,
	OldCurrency: 'static
		+ LockableCurrency<<T as frame_system::Config>::AccountId>
		+ Currency<<T as frame_system::Config>::AccountId>
		+ ReservableCurrency<<T as frame_system::Config>::AccountId>,
{
	_phantom: sp_std::marker::PhantomData<(T, OldCurrency)>,
}

impl<T, OldCurrency> frame_support::traits::OnRuntimeUpgrade for CustomOnRuntimeUpgrade<T, OldCurrency>
where
	T: Config,
	OldCurrency: 'static
		+ LockableCurrency<<T as frame_system::Config>::AccountId>
		+ Currency<<T as frame_system::Config>::AccountId>
		+ ReservableCurrency<<T as frame_system::Config>::AccountId>,
	BalanceOf<T>: From<OldCurrency::Balance>,
{
	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, DispatchError> {
		let active_collators = CandidatePool::<T>::get().0;

		for bond_info in active_collators {
			let owner = bond_info.owner;
			let balance = OldCurrency::free_balance(&owner);
			log::info!(
				"Collator: {:?} OldCurrency::free_balance pre_upgrade {:?}",
				HexDisplay::from(&owner.encode()),
				balance
			);
		}

		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), DispatchError> {
		let active_collators = CandidatePool::<T>::get().0;

		for bond_info in active_collators {
			let owner = bond_info.owner;
			let balance = OldCurrency::free_balance(&owner);
			log::info!(
				"Collator: {:?} OldCurrency::free_balance post_upgrade {:?}",
				HexDisplay::from(&owner.encode()),
				balance
			);
		}

		Ok(())
	}

	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		log::info!("Parachain Staking: on_runtime_upgrade");
		let mut read_ops = 0u64;
		let mut write_ops = 0u64;

		// Get all the active collators
		let active_collators = CandidatePool::<T>::get().0;
		read_ops += 1;

		for bond_info in active_collators {
			let owner = bond_info.owner;
			log::info!("Parachain Staking: migrating collator {:?}", HexDisplay::from(&owner.encode()));

			let candidate_info = CandidateInfo::<T>::get(&owner).unwrap();
			let bond_amount = candidate_info.bond;
			read_ops += 1;
			log::info!("Parachain Staking: bond_amount {:?}", bond_amount);

			let already_held: <T as Config>::Balance =
				T::Currency::balance_on_hold(&HoldReason::StakingCollator.into(), &owner);
			read_ops += 1;

			// Check if the lock is already held, to make migration idempotent
			if already_held == bond_amount {
				log::info!("Parachain Staking: already held {:?}", already_held);
			} else {
				// Remove the lock from the old currency
				OldCurrency::remove_lock(COLLATOR_LOCK_ID, &owner);
				write_ops += 1;

				// Hold the new currency
				T::Currency::hold(&HoldReason::StakingCollator.into(), &owner, bond_amount).unwrap_or_else(|err| {
					log::error!("Failed to add lock to parachain staking currency: {:?}", err);
				});
				write_ops += 1;

				// Get all the delegations for the collator
				if let Some(delegations) = TopDelegations::<T>::get(&owner) {
					read_ops += 1;
					for delegation in delegations.delegations {
						// Process each delegation
						log::info!(
							"Delegator: {:?}, Amount: {:?}",
							HexDisplay::from(&delegation.owner.encode()),
							delegation.amount
						);

						// Remove the lock from the old currency
						OldCurrency::remove_lock(DELEGATOR_LOCK_ID, &delegation.owner);
						write_ops += 1;

						// Hold the new currency
						T::Currency::hold(&HoldReason::StakingDelegator.into(), &delegation.owner, delegation.amount)
							.unwrap_or_else(|err| {
								log::error!("Failed to add lock to parachain staking currency: {:?}", err);
							});
						write_ops += 1;
					}
				} else {
					// Handle the case where there are no delegations for the account
					log::info!("No delegations found for the given account.");
				}
			}
		}

		log::info!("Parachain Staking: read_ops {:?} | write_ops: {:?}", read_ops, write_ops);

		<T as frame_system::Config>::DbWeight::get().reads_writes(read_ops, write_ops)
	}
}
