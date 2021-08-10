#![cfg(test)]

mod common;
use common::*;

use evm::{executor::PrecompileOutput, Context, ExitError, ExitSucceed};
use frame_support::{
	assert_noop, assert_ok,
	dispatch::Dispatchable,
	traits::{fungible::Inspect, PalletInfo, StorageInfo, StorageInfoTrait},
	weights::{DispatchClass, Weight},
	StorageHasher, Twox128,
};
use origintrail_parachain_runtime::{GLMR, RuntimeBlockWeights, Precompiles, ParachainStaking,
	Runtime, System, Event, AccountId, Balances, 
};
use nimbus_primitives::NimbusId;
use pallet_evm::PrecompileSet;
use pallet_transaction_payment::Multiplier;
use parachain_staking::{Bond, NominatorAdded};
use sha3::{Digest, Keccak256};
use sp_core::{Public, H160, U256};
use sp_runtime::{
	traits::{Convert, One},
	DispatchError,
};


#[test]
fn join_collator_candidates() {
	ExtBuilder::default()
		.with_balances(vec![
			(AccountId::from(ALICE), 2_000 * GLMR),
			(AccountId::from(BOB), 2_000 * GLMR),
			(AccountId::from(CHARLIE), 1_100 * GLMR),
			(AccountId::from(DAVE), 1_000 * GLMR),
		])
		.with_collators(vec![
			(AccountId::from(ALICE), 1_000 * GLMR),
			(AccountId::from(BOB), 1_000 * GLMR),
		])
		.with_nominations(vec![
			(AccountId::from(CHARLIE), AccountId::from(ALICE), 50 * GLMR),
			(AccountId::from(CHARLIE), AccountId::from(BOB), 50 * GLMR),
		])
		.build()
		.execute_with(|| {
			assert_noop!(
				ParachainStaking::join_candidates(
					origin_of(AccountId::from(ALICE)),
					1_000 * GLMR,
					2u32
				),
				parachain_staking::Error::<Runtime>::CandidateExists
			);
			assert_noop!(
				ParachainStaking::join_candidates(
					origin_of(AccountId::from(CHARLIE)),
					1_000 * GLMR,
					2u32
				),
				parachain_staking::Error::<Runtime>::NominatorExists
			);
			assert!(System::events().is_empty());
			assert_ok!(ParachainStaking::join_candidates(
				origin_of(AccountId::from(DAVE)),
				1_000 * GLMR,
				2u32
			));
			assert_eq!(
				last_event(),
				Event::ParachainStaking(parachain_staking::Event::JoinedCollatorCandidates(
					AccountId::from(DAVE),
					1_000 * GLMR,
					3_100 * GLMR
				))
			);
			let candidates = ParachainStaking::candidate_pool();
			assert_eq!(
				candidates.0[0],
				Bond {
					owner: AccountId::from(ALICE),
					amount: 1_050 * GLMR
				}
			);
			assert_eq!(
				candidates.0[1],
				Bond {
					owner: AccountId::from(BOB),
					amount: 1_050 * GLMR
				}
			);
			assert_eq!(
				candidates.0[2],
				Bond {
					owner: AccountId::from(DAVE),
					amount: 1_000 * GLMR
				}
			);
		});
}

#[test]
fn transfer_through_evm_to_stake() {
	ExtBuilder::default()
		.with_balances(vec![(AccountId::from(ALICE), 2_000 * GLMR)])
		.build()
		.execute_with(|| {
			// Charlie has no balance => fails to stake
			assert_noop!(
				ParachainStaking::join_candidates(
					origin_of(AccountId::from(CHARLIE)),
					1_000 * GLMR,
					2u32
				),
				DispatchError::Module {
					index: 3,
					error: 2,
					message: Some("InsufficientBalance")
				}
			);
			// Alice transfer from free balance 2000 GLMR to Bob
			assert_ok!(Balances::transfer(
				origin_of(AccountId::from(ALICE)),
				AccountId::from(BOB),
				2_000 * GLMR,
			));
			assert_eq!(Balances::free_balance(AccountId::from(BOB)), 2_000 * GLMR);

			let gas_limit = 100000u64;
			let gas_price: U256 = 1_000_000_000.into();
			// Bob transfers 1000 GLMR to Charlie via EVM
			assert_ok!(Call::EVM(pallet_evm::Call::<Runtime>::call(
				AccountId::from(BOB),
				AccountId::from(CHARLIE),
				Vec::new(),
				(1_000 * GLMR).into(),
				gas_limit,
				gas_price,
				None
			))
			.dispatch(<Runtime as frame_system::Config>::Origin::root()));
			assert_eq!(
				Balances::free_balance(AccountId::from(CHARLIE)),
				1_000 * GLMR,
			);

			// Charlie can stake now
			assert_ok!(ParachainStaking::join_candidates(
				origin_of(AccountId::from(CHARLIE)),
				1_000 * GLMR,
				2u32
			),);
			let candidates = ParachainStaking::candidate_pool();
			assert_eq!(
				candidates.0[0],
				Bond {
					owner: AccountId::from(CHARLIE),
					amount: 1_000 * GLMR
				}
			);
		});
}

/*
fn run_with_system_weight<F>(w: Weight, mut assertions: F)
where
	F: FnMut() -> (),
{
	let mut t: sp_io::TestExternalities = frame_system::GenesisConfig::default()
		.build_storage::<Runtime>()
		.unwrap()
		.into();
	t.execute_with(|| {
		System::set_block_consumed_resources(w, 0);
		assertions()
	});
}

#[test]
fn multiplier_can_grow_from_zero() {
	let minimum_multiplier = origintrail_parachain_runtime::MinimumMultiplier::get();
	let target = origintrail_parachain_runtime::TargetBlockFullness::get()
		* RuntimeBlockWeights::get()
			.get(DispatchClass::Normal)
			.max_total
			.unwrap();
	// if the min is too small, then this will not change, and we are doomed forever.
	// the weight is 1/100th bigger than target.
	run_with_system_weight(target * 101 / 100, || {
		let next =
			origintrail_parachain_runtime::SlowAdjustingFeeUpdate::<Runtime>::convert(minimum_multiplier);
		assert!(
			next > minimum_multiplier,
			"{:?} !>= {:?}",
			next,
			minimum_multiplier
		);
	})
}

#[test]
#[ignore] // test runs for a very long time
fn multiplier_growth_simulator() {
	// assume the multiplier is initially set to its minimum. We update it with values twice the
	//target (target is 25%, thus 50%) and we see at which point it reaches 1.
	let mut multiplier = origintrail_parachain_runtime::MinimumMultiplier::get();
	let block_weight = origintrail_parachain_runtime::TargetBlockFullness::get()
		* RuntimeBlockWeights::get()
			.get(DispatchClass::Normal)
			.max_total
			.unwrap()
		* 2;
	let mut blocks = 0;
	while multiplier <= Multiplier::one() {
		run_with_system_weight(block_weight, || {
			let next = origintrail_parachain_runtime::SlowAdjustingFeeUpdate::<Runtime>::convert(multiplier);
			// ensure that it is growing as well.
			assert!(next > multiplier, "{:?} !>= {:?}", next, multiplier);
			multiplier = next;
		});
		blocks += 1;
		println!("block = {} multiplier {:?}", blocks, multiplier);
	}
}
*/