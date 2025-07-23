use super::*;
use frame_support::{
    pallet_prelude::*,
    traits::{
        fungible::{Inspect as FungibleInspect, Mutate as FungibleMutate, Unbalanced as FungibleUnbalanced}, fungibles::{Dust, Inspect as FungiblesInspect, Mutate as FungiblesMutate, Unbalanced as FungiblesUnbalanced},
        tokens::{DepositConsequence, Fortitude, Precision, Preservation, Provenance, WithdrawConsequence},
        AsEnsureOriginWithArg,
    },
};
use serde::{Serialize, Deserialize};
use xcm::v3::{MultiLocation, Junction, NetworkId, Junctions};

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
            key: [0xaa, 0x7a, 0x9c, 0xa8, 0x7d, 0x36, 0x94, 0xb5, 0x75, 0x5f, 0x21, 0x3b, 0x5d, 0x04, 0x09, 0x4b, 0x8d, 0x0f, 0x0a, 0x6f]
        }
    )
};
pub const FOREIGN_TRAC_UNIFIED_ASSET_ID: UnifiedAssetId = UnifiedAssetId::Foreign(FOREIGN_TRAC_ASSET_LOCATION);

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
    pub const ApprovalDeposit: Balance = 0;
    pub const StringLimit: u32 = 50;
    pub const MetadataDepositBase: Balance = 10 * OTP;
    pub const MetadataDepositPerByte: Balance = 1 * OTP;
}

// Local Assets
impl pallet_assets::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = AssetId;
    type AssetIdParameter = codec::Compact<u128>;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
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
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    type RemoveItemsLimit = ConstU32<656>;
}

// Foreign Assets
impl pallet_assets::Config<pallet_assets::Instance2> for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Balance = Balance;
    type AssetId = MultiLocation;
    type AssetIdParameter = MultiLocation;
    type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
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
    type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
    type RemoveItemsLimit = ConstU32<656>;
}

// MulticurrencyAdapter
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Encode, Decode, TypeInfo, MaxEncodedLen, Serialize, Deserialize)]
pub enum UnifiedAssetId {
    Native,
    Local(u128),
    Foreign(MultiLocation), // now directly holds location
}

pub struct MultiCurrencyAdapter;
impl FungiblesInspect<AccountId> for MultiCurrencyAdapter {
    type AssetId = UnifiedAssetId;
    type Balance = u128;

    fn total_issuance(asset: UnifiedAssetId) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native =>
                <Balances as FungibleInspect<AccountId>>::total_issuance(),
            UnifiedAssetId::Local(id) =>
                <Assets as FungiblesInspect<AccountId>>::total_issuance(id.into()),
            UnifiedAssetId::Foreign(loc) =>
                <ForeignAssets as FungiblesInspect<AccountId>>::total_issuance(loc),
        }
    }

    fn minimum_balance(asset: UnifiedAssetId) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native => <Balances as FungibleInspect<AccountId>>::minimum_balance(),
            UnifiedAssetId::Local(id) => <Assets as FungiblesInspect<AccountId>>::minimum_balance(id.into()),
            UnifiedAssetId::Foreign(loc) => <ForeignAssets as FungiblesInspect<AccountId>>::minimum_balance(loc),
        }
    }

    fn balance(asset: UnifiedAssetId, who: &AccountId) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native => <Balances as FungibleInspect<AccountId>>::balance(who),
            UnifiedAssetId::Local(id) => <Assets as FungiblesInspect<AccountId>>::balance(id.into(), who),
            UnifiedAssetId::Foreign(loc) => <ForeignAssets as FungiblesInspect<AccountId>>::balance(loc, who),
        }
    }

    fn reducible_balance(
        asset: UnifiedAssetId,
        who: &AccountId,
        preservation: Preservation,
        force: Fortitude,
    ) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native => {
                <Balances as FungibleInspect<AccountId>>::reducible_balance(
                    who, preservation, force
                )
            },
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesInspect<AccountId>>::reducible_balance(
                    id.into(), who, preservation, force
                )
            },
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesInspect<AccountId>>::reducible_balance(
                    loc, who, preservation, force
                )
            },
        }
    }

    fn can_deposit(
        asset: UnifiedAssetId,
        who: &AccountId,
        amount: Self::Balance,
        mint: Provenance,
    ) -> DepositConsequence {
        match asset {
            UnifiedAssetId::Native =>
                <Balances as FungibleInspect<AccountId>>::can_deposit(who, amount, mint),
            UnifiedAssetId::Local(id) =>
                <Assets as FungiblesInspect<AccountId>>::can_deposit(id.into(), who, amount, mint),
            UnifiedAssetId::Foreign(loc) =>
                <ForeignAssets as FungiblesInspect<AccountId>>::can_deposit(loc, who, amount, mint),
        }
    }

    fn can_withdraw(
        asset: UnifiedAssetId,
        who: &AccountId,
        amount: Self::Balance,
    ) -> WithdrawConsequence<Self::Balance> {
        match asset {
            UnifiedAssetId::Native =>
                <Balances as FungibleInspect<AccountId>>::can_withdraw(who, amount),
            UnifiedAssetId::Local(id) =>
                <Assets as FungiblesInspect<AccountId>>::can_withdraw(id.into(), who, amount),
            UnifiedAssetId::Foreign(loc) =>
                <ForeignAssets as FungiblesInspect<AccountId>>::can_withdraw(loc, who, amount),
        }
    }

    fn asset_exists(asset: UnifiedAssetId) -> bool {
        match asset {
            UnifiedAssetId::Native => true,
            UnifiedAssetId::Local(id) => <Assets as FungiblesInspect<AccountId>>::asset_exists(id.into()),
            UnifiedAssetId::Foreign(loc) => <ForeignAssets as FungiblesInspect<AccountId>>::asset_exists(loc),
        }
    }

    fn total_balance(asset: UnifiedAssetId, who: &AccountId) -> Self::Balance {
        match asset {
            UnifiedAssetId::Native =>
                <Balances as FungibleInspect<AccountId>>::total_balance(who),
            UnifiedAssetId::Local(id) =>
                <Assets as FungiblesInspect<AccountId>>::total_balance(id.into(), who),
            UnifiedAssetId::Foreign(loc) =>
                <ForeignAssets as FungiblesInspect<AccountId>>::total_balance(loc, who),
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
            UnifiedAssetId::Native => <Balances as FungibleMutate<AccountId>>::mint_into(who, amount),
            UnifiedAssetId::Local(id) => <Assets as FungiblesMutate<AccountId>>::mint_into(id.into(), who, amount),
            UnifiedAssetId::Foreign(loc) => <ForeignAssets as FungiblesMutate<AccountId>>::mint_into(loc, who, amount),
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
                <Balances as FungibleMutate<AccountId>>::burn_from(
                    who, amount, precision, force
                )
            },
            UnifiedAssetId::Local(id) => <Assets as FungiblesMutate<AccountId>>::burn_from(id.into(), who, amount, precision, force),
            UnifiedAssetId::Foreign(loc) => <ForeignAssets as FungiblesMutate<AccountId>>::burn_from(loc, who, amount, precision, force),
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
            UnifiedAssetId::Native => {
                <Balances as FungibleMutate<AccountId>>::transfer(
                    source,
                    dest,
                    amount,
                    preservation
                )
            },
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesMutate<AccountId>>::transfer(
                    id.into(), source, dest, amount, preservation
                )
            },
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesMutate<AccountId>>::transfer(
                    loc, source, dest, amount, preservation
                )
            }
        }
    }

    fn set_balance(
        asset: UnifiedAssetId,
        who: &AccountId,
        amount: Self::Balance,
    ) -> Self::Balance {
        let current = Self::balance(asset.clone(), who);

        match current.cmp(&amount) {
            core::cmp::Ordering::Greater => {
                if let Err(e) = Self::burn_from(asset, who, current - amount, Precision::BestEffort, Fortitude::Force) {
                    log::warn!("Failed to burn excess balance: {:?}", e);
                }
            },
            core::cmp::Ordering::Less => {
                if let Err(e) = Self::mint_into(asset, who, amount - current) {
                    log::warn!("Failed to mint required balance: {:?}", e);
                }
            },
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
            },
            UnifiedAssetId::Local(id) => {
                <Assets as FungiblesUnbalanced<AccountId>>::write_balance(id.into(), who, amount)
            },
            UnifiedAssetId::Foreign(loc) => {
                <ForeignAssets as FungiblesUnbalanced<AccountId>>::write_balance(loc, who, amount)
            },
        }
    }

    fn set_total_issuance(asset: UnifiedAssetId, amount: Self::Balance) {
        match asset {
            UnifiedAssetId::Native =>
                <Balances as FungibleUnbalanced<AccountId>>::set_total_issuance(amount),
            UnifiedAssetId::Local(id) =>
                <Assets as FungiblesUnbalanced<AccountId>>::set_total_issuance(id.into(), amount),
            UnifiedAssetId::Foreign(loc) =>
                <ForeignAssets as FungiblesUnbalanced<AccountId>>::set_total_issuance(loc, amount),
        }
    }
}
