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

#![cfg(test)]

use frame_support::{
    construct_runtime, parameter_types,
    traits::{AsEnsureOriginWithArg, ConstU32},
    PalletId,
};
use frame_system as system;
use frame_system::EnsureSigned;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};
use sp_std::{vec, vec::Vec};

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;
type Balance = u128;
type AssetId = u32;

// Configure a mock runtime to test the pallet.
construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Assets: pallet_assets,
        Wrapper: crate,
    }
);

parameter_types! {
    pub const BlockHashCount: u64 = 250;
    pub const SS58Prefix: u8 = 42;
}

impl system::Config for Test {
    type BaseCallFilter = frame_support::traits::Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = u64;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = SS58Prefix;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type RuntimeTask = ();
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

parameter_types! {
    pub const AssetDeposit: u32 = 100;
    pub const ApprovalDeposit: u32 = 1;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: u32 = 10;
    pub const MetadataDepositPerByte: u32 = 1;
}

impl pallet_assets::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type AssetIdParameter = codec::Compact<AssetId>;
    type Currency = (); // Not used in our tests
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
    type ForceOrigin = frame_system::EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = ConstU32<1>;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = ();
    type RemoveItemsLimit = ConstU32<656>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ();
}

parameter_types! {
    pub const WrapperPalletId: PalletId = PalletId(*b"pwrapper");
    pub const LocalTracAssetId: AssetId = 1;
    pub const ForeignTracAssetId: AssetId = 2;
}

impl crate::Config for Test {
    type RuntimeEvent = RuntimeEvent;
    type Currency = pallet_assets::Pallet<Test>;
    type AssetId = AssetId;
    type Balance = Balance;
    type LocalTracAssetId = LocalTracAssetId;
    type ForeignTracAssetId = ForeignTracAssetId;
    type PalletId = WrapperPalletId;
    type PauseOrigin = frame_system::EnsureRoot<AccountId>;
    type WeightInfo = ();
}

// Test accounts
pub const ALICE: AccountId = 1;
pub const BOB: AccountId = 2;

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::<Test>::default()
        .build_storage()
        .unwrap();

    pallet_assets::GenesisConfig::<Test> {
        assets: vec![
            // Local TRAC asset
            (LocalTracAssetId::get(), ALICE, true, 1),
            // Foreign TRAC asset
            (ForeignTracAssetId::get(), ALICE, true, 1),
        ],
        metadata: vec![
            (
                LocalTracAssetId::get(),
                "Local TRAC".into(),
                "LTRAC".into(),
                18,
            ),
            (
                ForeignTracAssetId::get(),
                "Foreign TRAC".into(),
                "FTRAC".into(),
                18,
            ),
        ],
        accounts: vec![
            // Give ALICE some foreign TRAC
            (ForeignTracAssetId::get(), ALICE, 1000000),
            // Give BOB some foreign TRAC
            (ForeignTracAssetId::get(), BOB, 500000),
        ],
        next_asset_id: None,
    }
    .assimilate_storage(&mut t)
    .unwrap();

    let mut ext = sp_io::TestExternalities::new(t);
    ext.execute_with(|| System::set_block_number(1));
    ext
}
