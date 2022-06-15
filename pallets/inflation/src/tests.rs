#![cfg(test)]
#![allow(clippy::from_over_into)]
use crate as pallet_inflation;

use frame_support::{
	assert_ok, parameter_types,
	traits::{Currency, OnInitialize, Everything, ConstU32},
};
use frame_system::RawOrigin;
use sp_core::H256;
use sp_runtime::{
	traits::{BlakeTwo256, BlockNumberProvider, IdentityLookup},
	testing::Header,
};

use sp_runtime::{
	Perbill,
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

const YEAR: u64 = 2_629_800; // 12-second blocks
const FIRST_YEAR_BLOCK_INFLATION: u64 = 950;

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Test {
	type AccountStore = System;
	type Balance = u64;
	type DustRemoval = ();
	type Event = ();
	type ExistentialDeposit = ExistentialDeposit;
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = ();
}

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		Balances: pallet_balances::{Pallet, Call, Storage},
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Inflation: pallet_inflation::{Pallet, Call, Storage},
	}
);

parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
	pub const SS58Prefix: u8 = 101;
}

impl frame_system::Config for Test {
	type BaseCallFilter = Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = Call;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = SS58Prefix;
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
}

parameter_types! {
	pub FutureAuctionTreasuryId: u64 = 1111;
	pub CollatorsIncentivesTreasuryId: u64 = 2222;
	pub DkgIncentivesTreasuryId: u64 = 3333;
	pub CommunityTreasuryId: u64 = 4444;
	pub const InflationBlockInterval: u32 = 100; // every time per how many blocks inflation is applied
	pub static MockBlockNumberProvider: u64 = 0;
}

impl BlockNumberProvider for MockBlockNumberProvider {
	type BlockNumber = u64;

	fn current_block_number() -> Self::BlockNumber {
		Self::get()
	}
}

impl pallet_inflation::Config for Test {
	type Currency = Balances;
	type FutureAuctionTreasuryId = FutureAuctionTreasuryId;
	type CollatorsIncentivesTreasuryId = CollatorsIncentivesTreasuryId;
	type DkgIncentivesTreasuryId = DkgIncentivesTreasuryId;
	type CommunityTreasuryId = CommunityTreasuryId;
	type InflationBlockInterval = InflationBlockInterval;
	type BlockNumberProvider = MockBlockNumberProvider;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap()
		.into()
}

macro_rules! block_inflation {
	// Block inflation doesn't have any argumets
	() => {
		// Return BlockInflation state variable current value
		<pallet_inflation::BlockInflation<Test>>::get()
	};
}

#[test]
fn uninitialized_inflation() {
	new_test_ext().execute_with(|| {
		let initial_issuance: u64 = 500_000_000;
		let _ = <Balances as Currency<_>>::deposit_creating(&1234, initial_issuance);
		assert_eq!(Balances::free_balance(1234), initial_issuance);

		// BlockInflation should be set after inflation is started
		// first inflation deposit should be equal to BlockInflation
		MockBlockNumberProvider::set(1);

		assert_eq!(block_inflation!(), 0);
	});
}

#[test]
fn inflation_works() {
	new_test_ext().execute_with(|| {
		// Total issuance = 500_000_000
		let initial_issuance: u64 = 500_000_000;
		let _ = <Balances as Currency<_>>::deposit_creating(&1234, initial_issuance);
		assert_eq!(Balances::free_balance(1234), initial_issuance);

		// BlockInflation should be set after inflation is started
		// first inflation deposit should be equal to BlockInflation
		MockBlockNumberProvider::set(1);

		// Start inflation as sudo
		assert_ok!(Inflation::start_inflation(RawOrigin::Root.into(), 1));
		assert_eq!(block_inflation!(), FIRST_YEAR_BLOCK_INFLATION);
		assert_eq!(
			initial_issuance,
			<pallet_inflation::TotalInflation<Test>>::get() + block_inflation!()
		);

		// Trigger inflation
		MockBlockNumberProvider::set(102);
		Inflation::on_initialize(0);
		assert_eq!(
			initial_issuance,
			<pallet_inflation::TotalInflation<Test>>::get() + 2 * block_inflation!()
		);
	});
}

#[test]
fn inflation_rate_by_year() {
	new_test_ext().execute_with(|| {
		let initial_issuance: u64 = 500_000_000;
		let mut total_inflation: u64 = 500_000_000;
		let mut community_treasury: u64 = 0;
		let mut other_treasuries: u64 = 0;
		let mut next_block: bool = false;
		let _ = <Balances as Currency<_>>::deposit_creating(&1234, initial_issuance);
		assert_eq!(Balances::free_balance(1234), initial_issuance);

		println!("Initial inflation is {0} OTPs", total_inflation);
		println!("Starting inflation with 5% per year with 100 block frequency where 1 year has {0} blocks (12s)", YEAR);
		println!("");

		// Start inflation as sudo
		assert_ok!(Inflation::start_inflation(RawOrigin::Root.into(), 1));

		for year in 0..=9 {
			for block in 0..=YEAR {
				MockBlockNumberProvider::set((year * YEAR) + block);
				Inflation::on_initialize(0);

				if next_block == true {
					other_treasuries += Perbill::from_rational(InflationBlockInterval::get(), YEAR.try_into().unwrap()) * Perbill::from_percent(5) * Perbill::from_percent(30) * total_inflation;
					community_treasury += Perbill::from_rational(InflationBlockInterval::get(), YEAR.try_into().unwrap()) * Perbill::from_percent(5) * Perbill::from_percent(10) * total_inflation;

					assert_eq!(<Balances as Currency<_>>::free_balance(&FutureAuctionTreasuryId::get()),
							   other_treasuries);
					assert_eq!(<Balances as Currency<_>>::free_balance(&CollatorsIncentivesTreasuryId::get()),
							   other_treasuries);
					assert_eq!(<Balances as Currency<_>>::free_balance(&DkgIncentivesTreasuryId::get()),
							   other_treasuries);
					assert_eq!(<Balances as Currency<_>>::free_balance(&CommunityTreasuryId::get()),
							   community_treasury);

					next_block = false;
				}

				if (year * YEAR) + block % InflationBlockInterval::get() as u64 == 0 {
					next_block = true;
				}
			}

			total_inflation -= 3 * other_treasuries + community_treasury;

			println!("After year {0} inflation per 100 blocks is {1}", year + 1, block_inflation!());
			println!("After year {0} total inflation left is {1}", year + 1, total_inflation);
			println!("After year {0} total inflation for future auction treasury is {1}", year + 1, <Balances as Currency<_>>::free_balance(&FutureAuctionTreasuryId::get()));
			println!("After year {0} total inflation for collators incentives treasury is {1}", year + 1, <Balances as Currency<_>>::free_balance(&CollatorsIncentivesTreasuryId::get()));
			println!("After year {0} total inflation for dkg incentives treasury is {1}", year + 1, <Balances as Currency<_>>::free_balance(&DkgIncentivesTreasuryId::get()));
			println!("After year {0} total inflation for community treasury is {1}", year + 1, <Balances as Currency<_>>::free_balance(&CommunityTreasuryId::get()));
			println!("");

			next_block = false;
		}
	});
}