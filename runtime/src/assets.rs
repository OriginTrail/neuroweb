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

use super::*;
use frame_support::{
    pallet_prelude::*,
    traits::{
        fungible::{
            Inspect as FungibleInspect, Mutate as FungibleMutate, Unbalanced as FungibleUnbalanced,
        },
        fungibles::{
            Dust, Inspect as FungiblesInspect, Mutate as FungiblesMutate,
            Unbalanced as FungiblesUnbalanced,
        },
        tokens::{
            DepositConsequence, Fortitude, Precision, Preservation, Provenance, WithdrawConsequence,
        },
    },
};
use primitives::UnifiedAssetId;
use xcm::v3::{Junction, Junctions, MultiLocation, NetworkId};

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = OTP;

pub const LOCAL_TRAC_ASSET_ID: u128 = 1;
pub const LOCAL_TRAC_UNIFIED_ASSET_ID: UnifiedAssetId = UnifiedAssetId::Local(LOCAL_TRAC_ASSET_ID);

pub const FOREIGN_TRAC_ASSET_LOCATION: MultiLocation = MultiLocation {
    parents: 2,
    interior: Junctions::X2(
        Junction::GlobalConsensus(NetworkId::Ethereum { chain_id: 1 }),
        Junction::AccountKey20 {
            network: None,
            key: [
                0xaa, 0x7a, 0x9c, 0xa8, 0x7d, 0x36, 0x94, 0xb5, 0x75, 0x5f, 0x21, 0x3b, 0x5d, 0x04,
                0x09, 0x4b, 0x8d, 0x0f, 0x0a, 0x6f,
            ],
        },
    ),
};
pub const FOREIGN_TRAC_UNIFIED_ASSET_ID: UnifiedAssetId =
    UnifiedAssetId::Foreign(FOREIGN_TRAC_ASSET_LOCATION);

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

// Native Asset
impl pallet_balances::Config for Runtime {
    type MaxLocks = ConstU32<50>;
    /// The type for recording an account's balance.
    type Balance = Balance;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ExistentialDeposit;
    type AccountStore = System;
    type WeightInfo = pallet_balances::weights::SubstrateWeight<Runtime>;
    type MaxReserves = ConstU32<50>;
    type ReserveIdentifier = [u8; 8];
    type RuntimeHoldReason = RuntimeHoldReason;
    type RuntimeFreezeReason = RuntimeFreezeReason;
    type FreezeIdentifier = ();
    type MaxFreezes = ConstU32<1>;
}

parameter_types! {
    pub const AssetDeposit: Balance = 100 * OTP;
    pub const AssetAccountDeposit: Balance = 100 * OTP;
    pub const ApprovalDeposit: Balance = 100 * OTP;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 10 * OTP;
    pub const MetadataDepositPerByte: Balance = 1 * OTP;
    pub const LocalAssetsPalletId: PalletId = PalletId(*b"p/locass");
    pub const ForeignAssetsPalletId: PalletId = PalletId(*b"p/fgnass");
}

// Local Assets
impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type RemoveItemsLimit = ConstU32<656>;
    type AssetId = AssetId;
    type AssetIdParameter = codec::Compact<u128>;
    type Currency = Balances;
    type CreateOrigin = RootWithLocalAssetsPalletAccount;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = weights::pallet_assets_local::NeurowebWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = AssetsBenchmarkHelper;
}

// Foreign Assets
impl pallet_assets::Config<pallet_assets::Instance2> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type RemoveItemsLimit = ConstU32<656>;
    type AssetId = MultiLocation;
    type AssetIdParameter = MultiLocation;
    type Currency = Balances;
    type CreateOrigin = RootWithForeignAssetsPalletsAccount;
    type ForceOrigin = EnsureRoot<AccountId>;
    type AssetDeposit = AssetDeposit;
    type AssetAccountDeposit = AssetAccountDeposit;
    type MetadataDepositBase = MetadataDepositBase;
    type MetadataDepositPerByte = MetadataDepositPerByte;
    type ApprovalDeposit = ApprovalDeposit;
    type StringLimit = StringLimit;
    type Freezer = ();
    type Extra = ();
    type CallbackHandle = ();
    type WeightInfo = weights::pallet_assets_foreign::NeurowebWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type BenchmarkHelper = ForeignAssetsBenchmarkHelper;
}

pub struct MultiCurrencyAdapter;
impl FungiblesInspect<AccountId> for MultiCurrencyAdapter {
    type AssetId = UnifiedAssetId;
    type Balance = u128;

    fn total_issuance(asset: UnifiedAssetId) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native => <Balances as FungibleInspect<AccountId>>::total_issuance(),
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::total_issuance(id.into())
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::total_issuance(loc)
            }
        }
    }

    fn minimum_balance(asset: UnifiedAssetId) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native => <Balances as FungibleInspect<AccountId>>::minimum_balance(),
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::minimum_balance(id.into())
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::minimum_balance(loc)
            }
        }
    }

    fn balance(asset: UnifiedAssetId, who: &AccountId) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native => <Balances as FungibleInspect<AccountId>>::balance(who),
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::balance(id.into(), who)
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::balance(loc, who)
            }
        }
    }

    fn reducible_balance(
        asset: UnifiedAssetId,
        who: &AccountId,
        preservation: Preservation,
        force: Fortitude,
    ) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native => <Balances as FungibleInspect<AccountId>>::reducible_balance(
                who,
                preservation,
                force,
            ),
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::reducible_balance(
                    id.into(),
                    who,
                    preservation,
                    force,
                )
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::reducible_balance(
                    loc,
                    who,
                    preservation,
                    force,
                )
            }
        }
    }

    fn can_deposit(
        asset: UnifiedAssetId,
        who: &AccountId,
        amount: Self::Balance,
        mint: Provenance,
    ) -> DepositConsequence {
        match asset {
            UnifiedAssetId::Native => {
                <Balances as FungibleInspect<AccountId>>::can_deposit(who, amount, mint)
            }
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::can_deposit(id.into(), who, amount, mint)
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::can_deposit(loc, who, amount, mint)
            }
        }
    }

    fn can_withdraw(
        asset: UnifiedAssetId,
        who: &AccountId,
        amount: Self::Balance,
    ) -> WithdrawConsequence<Self::Balance> {
        match asset {
            UnifiedAssetId::Native => {
                <Balances as FungibleInspect<AccountId>>::can_withdraw(who, amount)
            }
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::can_withdraw(id.into(), who, amount)
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::can_withdraw(loc, who, amount)
            }
        }
    }

    fn asset_exists(asset: UnifiedAssetId) -> bool {
        match asset {
            UnifiedAssetId::Native => true,
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::asset_exists(id.into())
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::asset_exists(loc)
            }
        }
    }

    fn total_balance(asset: UnifiedAssetId, who: &AccountId) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native => <Balances as FungibleInspect<AccountId>>::total_balance(who),
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::total_balance(id.into(), who)
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::total_balance(loc, who)
            }
        }
    }
}

impl FungiblesMutate<AccountId> for MultiCurrencyAdapter {
    fn mint_into(
        asset: UnifiedAssetId,
        who: &AccountId,
        amount: Self::Balance,
    ) -> Result<Self::Balance, DispatchError> {
        match asset {
            UnifiedAssetId::Native => {
                <Balances as FungibleMutate<AccountId>>::mint_into(who, amount)
            }
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesMutate<AccountId>>::mint_into(id.into(), who, amount)
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesMutate<AccountId>>::mint_into(loc, who, amount)
            }
        }
    }

    fn burn_from(
        asset: UnifiedAssetId,
        who: &AccountId,
        amount: Self::Balance,
        precision: Precision,
        force: Fortitude,
    ) -> Result<Self::Balance, DispatchError> {
        match asset {
            UnifiedAssetId::Native => {
                <Balances as FungibleMutate<AccountId>>::burn_from(who, amount, precision, force)
            }
            UnifiedAssetId::Local(id) => <Assets as FungiblesMutate<AccountId>>::burn_from(
                id.into(),
                who,
                amount,
                precision,
                force,
            ),
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesMutate<AccountId>>::burn_from(
                    loc, who, amount, precision, force,
                )
            }
        }
    }

    fn transfer(
        asset: UnifiedAssetId,
        source: &AccountId,
        dest: &AccountId,
        amount: Self::Balance,
        preservation: Preservation,
    ) -> Result<Self::Balance, DispatchError> {
        match asset {
            UnifiedAssetId::Native => <Balances as FungibleMutate<AccountId>>::transfer(
                source,
                dest,
                amount,
                preservation,
            ),
            UnifiedAssetId::Local(id) => <Assets as FungiblesMutate<AccountId>>::transfer(
                id.into(),
                source,
                dest,
                amount,
                preservation,
            ),
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesMutate<AccountId>>::transfer(
                    loc,
                    source,
                    dest,
                    amount,
                    preservation,
                )
            }
        }
    }

    fn set_balance(asset: UnifiedAssetId, who: &AccountId, amount: Self::Balance) -> Self::Balance {
        let current = Self::balance(asset.clone(), who);

        match current.cmp(&amount) {
            core::cmp::Ordering::Greater => {
                if let Err(e) = Self::burn_from(
                    asset,
                    who,
                    current - amount,
                    Precision::BestEffort,
                    Fortitude::Force,
                ) {
                    log::warn!("Failed to burn excess balance: {:?}", e);
                }
            }
            core::cmp::Ordering::Less => {
                if let Err(e) = Self::mint_into(asset, who, amount - current) {
                    log::warn!("Failed to mint required balance: {:?}", e);
                }
            }
            core::cmp::Ordering::Equal => {
                // Already at target balance
            }
        }

        amount
    }
}

impl FungiblesUnbalanced<AccountId> for MultiCurrencyAdapter {
    fn handle_dust(_dust: Dust<AccountId, Self>) {
        // No-op: You can log it, burn it, or track it if needed
    }

    fn write_balance(
        asset: UnifiedAssetId,
        who: &AccountId,
        amount: Self::Balance,
    ) -> Result<Option<Self::Balance>, DispatchError> {
        match asset {
            UnifiedAssetId::Native => {
                <Balances as FungibleUnbalanced<AccountId>>::write_balance(who, amount)
            }
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesUnbalanced<AccountId>>::write_balance(id.into(), who, amount)
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesUnbalanced<AccountId>>::write_balance(loc, who, amount)
            }
        }
    }

    fn set_total_issuance(asset: UnifiedAssetId, amount: Self::Balance) {
        match asset {
            UnifiedAssetId::Native => {
                <Balances as FungibleUnbalanced<AccountId>>::set_total_issuance(amount)
            }
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesUnbalanced<AccountId>>::set_total_issuance(id.into(), amount)
            }
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesUnbalanced<AccountId>>::set_total_issuance(loc, amount)
            }
        }
    }
}

pub fn local_assets_pallet_account() -> AccountId {
    LocalAssetsPalletId::get().into_account_truncating()
}

pub fn foreign_assets_pallet_account() -> AccountId {
    ForeignAssetsPalletId::get().into_account_truncating()
}

// CreateOrigin for local assets
// Root which returns the local_assets_pallet_account
pub struct RootWithLocalAssetsPalletAccount;
impl frame_support::traits::EnsureOriginWithArg<RuntimeOrigin, u128>
for RootWithLocalAssetsPalletAccount
{
    type Success = AccountId;
    fn try_origin(o: RuntimeOrigin, _asset_id: &u128) -> Result<AccountId, RuntimeOrigin> {
        <EnsureRoot<AccountId> as frame_support::traits::EnsureOriginWithArg<RuntimeOrigin, u128>>::try_origin(o, _asset_id).map(|_| local_assets_pallet_account())
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin(_asset_id: &u128) -> Result<RuntimeOrigin, ()> {
        Ok(frame_system::RawOrigin::Root.into())
    }
}

// CreateOrigin for foreign assets
// Root which returns the foreign_assets_pallet_account
pub struct RootWithForeignAssetsPalletsAccount;
impl frame_support::traits::EnsureOriginWithArg<RuntimeOrigin, MultiLocation>
for RootWithForeignAssetsPalletsAccount
{
    type Success = AccountId;
    fn try_origin(o: RuntimeOrigin, _asset_id: &MultiLocation) -> Result<AccountId, RuntimeOrigin> {
        <EnsureRoot<AccountId> as frame_support::traits::EnsureOriginWithArg<
            RuntimeOrigin,
            MultiLocation,
        >>::try_origin(o, _asset_id)
            .map(|_| foreign_assets_pallet_account())
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn try_successful_origin(_asset_id: &MultiLocation) -> Result<RuntimeOrigin, ()> {
        Ok(frame_system::RawOrigin::Root.into())
    }
}

// Benchmarking helpers
#[cfg(feature = "runtime-benchmarks")]
pub struct AssetsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<codec::Compact<u128>> for AssetsBenchmarkHelper {
    fn create_asset_id_parameter(id: u32) -> codec::Compact<u128> {
        (id as u128).into()
    }
}

#[cfg(feature = "runtime-benchmarks")]
pub struct ForeignAssetsBenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_assets::BenchmarkHelper<MultiLocation> for ForeignAssetsBenchmarkHelper {
    fn create_asset_id_parameter(id: u32) -> MultiLocation {
        MultiLocation::new(1, Junction::Parachain(id))
    }
}
