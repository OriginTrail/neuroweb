#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

mod weights;
pub mod xcm_config;
pub use frame_support::traits::Get;

use cumulus_pallet_parachain_system::RelayNumberMonotonicallyIncreases;
use smallvec::smallvec;
use sp_api::impl_runtime_apis;
use sp_core::{crypto::KeyTypeId, OpaqueMetadata, H160, H256, U256};
use sp_runtime::{
    create_runtime_str, generic, impl_opaque_keys, DispatchResult,
    traits::{
        AccountIdConversion, IdentityLookup, BlakeTwo256, Block as BlockT,
        ConvertInto, DispatchInfoOf, Dispatchable, IdentifyAccount, 
        PostDispatchInfoOf, UniqueSaturatedInto, Verify, One
    },
    transaction_validity::{
        TransactionSource, TransactionValidity, TransactionValidityError
    },
    ApplyExtrinsicResult, MultiSignature, RuntimeDebug,
};
use sp_runtime::Percent;
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

use codec::{Encode, Decode, MaxEncodedLen};
use frame_support::{
    ord_parameter_types,
    construct_runtime, parameter_types, transactional,
    genesis_builder_helper::{build_state, get_preset},
    traits::{
        tokens::{PayFromAccount, UnityAssetBalanceConversion},
        fungible::{Balanced, Credit, HoldConsideration, Inspect},
        AsEnsureOriginWithArg, Currency as PalletCurrency, EqualPrivilegeOnly, EitherOfDiverse, 
        Everything, FindAuthor, ReservableCurrency, Imbalance, InstanceFilter, OnUnbalanced, ConstBool,
        ConstU128, ConstU32, ConstU64, ConstU8, WithdrawReasons, OnFinalize, LinearStoragePrice,
        ExistenceRequirement, TransformOrigin
    },
    dispatch::DispatchClass,
    weights::{
        constants::WEIGHT_REF_TIME_PER_SECOND,
        ConstantMultiplier, Weight, WeightToFeeCoefficient, WeightToFeeCoefficients,
        WeightToFeePolynomial,
    },
    ConsensusEngineId, PalletId,
};
use frame_system::{
    limits::{BlockLength, BlockWeights},
    EnsureRoot, EnsureSigned,
};
pub use sp_consensus_aura::sr25519::AuthorityId as AuraId;
pub use sp_runtime::{MultiAddress, Perbill, Permill};
use pallet_transaction_payment::{ConstFeeMultiplier, Multiplier};
use weights::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight};
use xcm_config::XcmOriginToTransactDispatchOrigin;

#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;

// Polkadot Imports
use polkadot_runtime_common::BlockHashCount;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use parachains_common::message_queue::{NarrowOriginToSibling, ParaIdToSibling};
use polkadot_runtime_common::xcm_sender::NoPriceForMessageDelivery;
pub use pallet_parachain_staking;

// XCM Imports
use xcm::latest::prelude::BodyId;

// Frontier
use pallet_evm::{
	EnsureAddressRoot, EnsureAddressNever, Account as EVMAccount, EVMFungibleAdapter,
    FeeCalculator, OnChargeEVMTransaction as OnChargeEVMTransactionT, Runner
};
use pallet_ethereum::{Call::transact, PostLogContent, EthereumBlockHashMapping, Transaction as EthereumTransaction};
use fp_rpc::TransactionStatus;
use pallet_evm_accounts::{EvmAddressMapping, MergeAccount};
use pallet_evm_precompile_assets_erc20::AddressToAssetId;
use pallet_identity::legacy::IdentityInfo;

mod precompiles;
use precompiles::{FrontierPrecompiles, ASSET_PRECOMPILE_ADDRESS_PREFIX};

/// Handles converting a weight scalar to a fee value, based on the scale and granularity of the
/// node's balance type.
///
/// This should typically create a mapping between the following ranges:
///   - `[0, MAXIMUM_BLOCK_WEIGHT]`
///   - `[Balance::min, Balance::max]`
///
/// Yet, it can be used for any other sort of change to weight-fee. Some examples being:
///   - Setting it to `0` will essentially disable the weight fee.
///   - Setting it to `1` will cause the literal `#[weight = x]` values to be charged.
pub struct WeightToFee;
impl WeightToFeePolynomial for WeightToFee {
    type Balance = Balance;
    fn polynomial() -> WeightToFeeCoefficients<Self::Balance> {
        // in Rococo, extrinsic base weight (smallest non-zero weight) is mapped to 1 MILLIOTP:
        // we map to 1/10 of that, or 1/10 MILLIOTP
        let p = MILLIOTP / 10;
        let q = 100 * Balance::from(ExtrinsicBaseWeight::get().ref_time());
        smallvec![WeightToFeeCoefficient {
            degree: 1,
            negative: false,
            coeff_frac: Perbill::from_rational(p % q, q),
            coeff_integer: p / q,
        }]
    }
}

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core data structures.
pub mod opaque {
    use super::*;
    use sp_runtime::{generic, traits::BlakeTwo256};

    pub use sp_runtime::OpaqueExtrinsic as UncheckedExtrinsic;
    /// Opaque block header type.
    pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
    /// Opaque block type.
    pub type Block = generic::Block<Header, UncheckedExtrinsic>;
    /// Opaque block identifier type.
    pub type BlockId = generic::BlockId<Block>;
}

impl_opaque_keys! {
    pub struct SessionKeys {
        pub aura: Aura,
    }
}

#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
    spec_name: create_runtime_str!("origintrail-parachain"),
    impl_name: create_runtime_str!("neuroweb"),
    authoring_version: 1,
    spec_version: 137,
    impl_version: 0,
    apis: RUNTIME_API_VERSIONS,
    transaction_version: 1,
    state_version: 1,
};

/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included into the
/// relay chain.
pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;

/// How many parachain blocks are processed by the relay chain per parent. Limits the number of
/// blocks authored per slot.
pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;
/// Relay chain slot duration, in milliseconds.
pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

/// This determines the average expected block time that we are targeting.
/// Blocks will be produced at a minimum duration defined by `SLOT_DURATION`.
/// `SLOT_DURATION` is picked up by `pallet_timestamp` which is in turn picked
/// up by `pallet_aura` to implement `fn slot_duration()`.
///
/// Change this to adjust the block time.
pub const MILLISECS_PER_BLOCK: u64 = 6000;

// NOTE: Currently it is not possible to change the slot duration after the chain has started.
//       Attempting to do so will brick block production.
pub const SLOT_DURATION: u64 = MILLISECS_PER_BLOCK;

// Time is measured by number of blocks.
pub const MINUTES: BlockNumber = 60_000 / (MILLISECS_PER_BLOCK as BlockNumber);
pub const HOURS: BlockNumber = MINUTES * 60;
pub const DAYS: BlockNumber = HOURS * 24;

// OTP = the base number of indivisible units for balances
pub const OTP: Balance = 1_000_000_000_000;
pub const MILLIOTP: Balance = 1_000_000_000;
pub const MICROOTP: Balance = 1_000_000;

/// The existential deposit. Set to 1/10 of the Connected Relay Chain.
pub const EXISTENTIAL_DEPOSIT: Balance = OTP;

/// We assume that ~5% of the block weight is consumed by `on_initialize` handlers. This is
/// used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(5);

/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used by
/// `Operational` extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);

/// We allow for 2 seconds of compute with a 6 second average block.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_mul(2),
	cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

pub const fn deposit(items: u32, bytes: u32) -> Balance {
    items as Balance * 15 * MILLIOTP + (bytes as Balance) * 6 * MILLIOTP
}

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
    NativeVersion {
        runtime_version: VERSION,
        can_author_with: Default::default(),
    }
}

parameter_types! {
    pub const Version: RuntimeVersion = VERSION;

    // This part is copied from Substrate's `bin/node/runtime/src/lib.rs`.
    //  The `RuntimeBlockLength` and `RuntimeBlockWeights` exist here because the
    // `DeletionWeightLimit` and `DeletionQueueDepth` depend on those to parameterize
    // the lazy contract deletion.
    pub RuntimeBlockLength: BlockLength =
        BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
    pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
        .base_block(BlockExecutionWeight::get())
        .for_class(DispatchClass::all(), |weights| {
            weights.base_extrinsic = ExtrinsicBaseWeight::get();
        })
        .for_class(DispatchClass::Normal, |weights| {
            weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
        })
        .for_class(DispatchClass::Operational, |weights| {
            weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
            // Operational transactions have some extra reserved space, so that they
            // are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
            weights.reserved = Some(
                MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
            );
        })
        .avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
        .build_or_panic();
    pub const SS58Prefix: u16 = 101;
}

// Pallet accounts of runtime
parameter_types! {
    pub const TreasuryPalletId: PalletId = PalletId(*b"otp/trea");
    pub const DkgIncentivesPalletId: PalletId = PalletId(*b"otp/dkgi");
    pub const FutureAuctionsPalletId: PalletId = PalletId(*b"otp/fauc");
    pub const CollatorsIncentivesPalletId: PalletId = PalletId(*b"otp/coli");
    pub const PotId: PalletId = PalletId(*b"PotStake");
}

// Configure FRAME pallets to include in runtime.

impl frame_system::Config for Runtime {
    /// The identifier used to distinguish between accounts.
    type AccountId = AccountId;
    /// The aggregated dispatch type that is available for extrinsics.
    type RuntimeCall = RuntimeCall;
    /// The lookup mechanism to get account ID from whatever is passed in dispatchers.
    type Lookup = IdentityLookup<AccountId>;
    /// The nonce type for storing how many extrinsics an account has signed.
    type Nonce = Nonce;
    /// The type for blocks.
    type Block = Block;
    /// The type for hashing blocks and tries.
    type Hash = Hash;
    /// The hashing algorithm used.
    type Hashing = BlakeTwo256;
    /// The ubiquitous event type.
    type RuntimeEvent = RuntimeEvent;
    /// The ubiquitous origin type.
    type RuntimeOrigin = RuntimeOrigin;
    /// The aggregated RuntimeTask type.
    type RuntimeTask = RuntimeTask;
    /// Maximum number of block number to block hash mappings to keep (oldest pruned first).
    type BlockHashCount = BlockHashCount;
    /// Runtime version.
    type Version = Version;
    /// Converts a module to an index of this module in the runtime.
    type PalletInfo = PalletInfo;
    /// The data to be stored in an account.
    type AccountData = pallet_balances::AccountData<Balance>;
    /// What to do if a new account is created.
    type OnNewAccount = ();
    /// What to do if an account is fully reaped from the system.
    type OnKilledAccount = ();
    /// The weight of database operations that the runtime can invoke.
    type DbWeight = RocksDbWeight;
    /// The basic call filter to use in dispatchable.
    type BaseCallFilter = Everything;
    /// Weight information for the extrinsics of this pallet.
    type SystemWeightInfo = ();
    /// Block & extrinsics weights: base values and limits.
    type BlockWeights = RuntimeBlockWeights;
    /// The maximum length of a block (in bytes).
    type BlockLength = RuntimeBlockLength;
    /// This is used as an identifier of the chain.
    type SS58Prefix = SS58Prefix;
    /// The action to take on a Runtime Upgrade
    type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
    type MaxConsumers = frame_support::traits::ConstU32<16>;
    type SingleBlockMigrations = ();
    type MultiBlockMigrator = ();
    type PreInherents = ();
    type PostInherents = ();
    type PostTransactions = ();
}

impl pallet_timestamp::Config for Runtime {
    /// A timestamp: milliseconds since the unix epoch.
    type Moment = u64;
    type OnTimestampSet = Aura;
    #[cfg(feature = "experimental")]
    type MinimumPeriod = ConstU64<0>;
    #[cfg(not(feature = "experimental"))]
    type MinimumPeriod = ConstU64<{ SLOT_DURATION / 2 }>;
    type WeightInfo = pallet_timestamp::weights::SubstrateWeight<Runtime>;
}

impl pallet_authorship::Config for Runtime {
    type FindAuthor = pallet_session::FindAccountFromAuthorIndex<Self, Aura>;
    type EventHandler = ParachainStaking;
}

parameter_types! {
    pub const ExistentialDeposit: Balance = EXISTENTIAL_DEPOSIT;
}

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

pub struct CollatorsIncentivesPot;
impl OnUnbalanced<Credit<AccountId, Balances>> for CollatorsIncentivesPot
{
    fn on_nonzero_unbalanced(amount: Credit<AccountId, Balances>) {
        let staking_pot = CollatorsIncentivesPalletId::get().into_account_truncating();
        let _ = Balances::resolve(&staking_pot, amount);
    }
}

pub struct FutureAuctionsPot;
impl OnUnbalanced<Credit<AccountId, Balances>> for FutureAuctionsPot
{
    fn on_nonzero_unbalanced(amount: Credit<AccountId, Balances>) {
        let future_auctions_pot = FutureAuctionsPalletId::get().into_account_truncating();
        let _ = Balances::resolve(&future_auctions_pot, amount);   
    }  
}

pub struct DkgIncentivesPot;
impl OnUnbalanced<Credit<AccountId, Balances>> for DkgIncentivesPot
{
    fn on_nonzero_unbalanced(amount: Credit<AccountId, Balances>) {
        let dkg_incentives_pot = DkgIncentivesPalletId::get().into_account_truncating();
        let _ = Balances::resolve(&dkg_incentives_pot, amount);
    }
}

pub struct TreasuryPot;
impl OnUnbalanced<Credit<AccountId, Balances>> for TreasuryPot
{
    fn on_nonzero_unbalanced(amount: Credit<AccountId, Balances>) {
        let treasury_port = TreasuryPalletId::get().into_account_truncating();
        let _ = Balances::resolve(&treasury_port, amount);
    }
}


pub struct DealWithFees;
impl OnUnbalanced<Credit<AccountId, Balances>> for DealWithFees
{
    // this is called for substrate-based transactions
    fn on_unbalanceds<B>(mut fees_then_tips: impl Iterator<Item = Credit<AccountId, Balances>>) {
        if let Some(mut fees) = fees_then_tips.next() {
            if let Some(tips) = fees_then_tips.next() {
                tips.merge_into(&mut fees);
            }
            // for fees and tips:
            // - 30% to DKG Incentives pool
            // - 30% to Collators Incentives pool (currently staking_pot)
            // - 30% to Future Auctions pool
            // - 10% to Treasury
            let split = fees.ration(60, 40);
            let (dkg_incentives_fees, collators_incentives_fees) = split.0.ration(50, 50);
            let (future_auctions_fees, treasury_fees) = split.1.ration(75, 25);

            <TreasuryPot as OnUnbalanced<_>>::on_unbalanced(treasury_fees);
            <CollatorsIncentivesPot as OnUnbalanced<_>>::on_unbalanced(collators_incentives_fees);
            <FutureAuctionsPot as OnUnbalanced<_>>::on_unbalanced(future_auctions_fees);
            <DkgIncentivesPot as OnUnbalanced<_>>::on_unbalanced(dkg_incentives_fees);
        }
    }

    // this is called from pallet_evm for Ethereum-based transactions
    // (technically, it calls on_unbalanced, which calls this when non-zero)
    fn on_nonzero_unbalanced(amount: Credit<AccountId, Balances>) {
        let split = amount.ration(60, 40);
        let (dkg_incentives_fees, collators_incentives_fees) = split.0.ration(50, 50);
        let (future_auctions_fees, treasury_fees) = split.1.ration(75, 25);


        <TreasuryPot as OnUnbalanced<_>>::on_unbalanced(treasury_fees);
        <CollatorsIncentivesPot as OnUnbalanced<_>>::on_unbalanced(collators_incentives_fees);
        <FutureAuctionsPot as OnUnbalanced<_>>::on_unbalanced(future_auctions_fees);
        <DkgIncentivesPot as OnUnbalanced<_>>::on_unbalanced(dkg_incentives_fees);
    }
}

parameter_types! {
    /// Relay Chain `TransactionByteFee` / 10
    pub const TransactionByteFee: Balance = 10 * MICROOTP;
    pub FeeMultiplier: Multiplier = Multiplier::one();
}

impl pallet_transaction_payment::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnChargeTransaction = pallet_transaction_payment::FungibleAdapter<Balances, DealWithFees>;
    type LengthToFee = ConstantMultiplier<Balance, TransactionByteFee>;
    type WeightToFee = WeightToFee;
    type FeeMultiplierUpdate = ConstFeeMultiplier<FeeMultiplier>;
    type OperationalFeeMultiplier = ConstU8<5>;
}

parameter_types! {
    pub const ReservedXcmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT.saturating_div(4);
    pub const RelayOrigin: AggregateMessageOrigin = AggregateMessageOrigin::Parent;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type OnSystemEvent = ();
    type SelfParaId = parachain_info::Pallet<Runtime>;
    type DmpQueue = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
    type ReservedDmpWeight = ReservedDmpWeight;
    type OutboundXcmpMessageSource = XcmpQueue;
    type XcmpMessageHandler = XcmpQueue;
    type ReservedXcmpWeight = ReservedXcmpWeight;
    type CheckAssociatedRelayNumber = RelayNumberMonotonicallyIncreases;
    type ConsensusHook = ConsensusHook;
    type WeightInfo = cumulus_pallet_parachain_system::weights::SubstrateWeight<Runtime>;
}
type ConsensusHook = cumulus_pallet_aura_ext::FixedVelocityConsensusHook<
    Runtime,
    RELAY_CHAIN_SLOT_DURATION_MILLIS,
    BLOCK_PROCESSING_VELOCITY,
    UNINCLUDED_SEGMENT_CAPACITY,
>;


impl parachain_info::Config for Runtime {}

impl cumulus_pallet_aura_ext::Config for Runtime {}

impl cumulus_pallet_xcmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ChannelInfo = ParachainSystem;
    type VersionWrapper = ();
    type XcmpQueue = TransformOrigin<MessageQueue, AggregateMessageOrigin, ParaId, ParaIdToSibling>;
    type MaxInboundSuspended = ConstU32<1_000>;
    type ControllerOrigin = EnsureRoot<AccountId>;
    type ControllerOriginConverter = XcmOriginToTransactDispatchOrigin;
    type WeightInfo = cumulus_pallet_xcmp_queue::weights::SubstrateWeight<Runtime>;
    type PriceForSiblingDelivery = NoPriceForMessageDelivery<ParaId>;
}

impl cumulus_pallet_dmp_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type DmpSink = frame_support::traits::EnqueueWithOrigin<MessageQueue, RelayOrigin>;
    type WeightInfo = cumulus_pallet_dmp_queue::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    	/// The amount of weight (if any) which should be provided to the message queue for
	/// servicing enqueued items.
	///
	/// This may be legitimately `None` in the case that you will call
	/// `ServiceQueues::service_queues` manually.
	pub MessageQueueServiceWeight: Weight =
    Perbill::from_percent(25) * RuntimeBlockWeights::get().max_block;
    /// The maximum number of stale pages (i.e. of overweight messages) allowed before culling
    /// can happen. Once there are more stale pages than this, then historical pages may be
    /// dropped, even if they contain unprocessed overweight messages.
    pub const MessageQueueMaxStale: u32 = 8;
    /// The size of the page; this implies the maximum message size which can be sent.
    ///
    /// A good value depends on the expected message sizes, their weights, the weight that is
    /// available for processing them and the maximal needed message size. The maximal message
    /// size is slightly lower than this as defined by [`MaxMessageLenOf`].
    pub const MessageQueueHeapSize: u32 = 128 * 1048;
}

impl pallet_message_queue::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = pallet_message_queue::weights::SubstrateWeight<Runtime>;
    #[cfg(feature = "runtime-benchmarks")]
    type MessageProcessor = pallet_message_queue::mock_helpers::NoopMessageProcessor<
        cumulus_primitives_core::AggregateMessageOrigin,
    >;
    #[cfg(not(feature = "runtime-benchmarks"))]
    type MessageProcessor = xcm_builder::ProcessXcmMessage<
        AggregateMessageOrigin,
        xcm_executor::XcmExecutor<xcm_config::XcmConfig>,
        RuntimeCall,
    >;
    type Size = u32;
    type QueueChangeHandler = NarrowOriginToSibling<XcmpQueue>;
    type QueuePausedQuery = NarrowOriginToSibling<XcmpQueue>;
    type HeapSize = MessageQueueHeapSize;
    type MaxStale = MessageQueueMaxStale;
    type ServiceWeight = MessageQueueServiceWeight;
    type IdleMaxServiceWeight = MessageQueueServiceWeight;
}

parameter_types! {
    pub const DepositBase: Balance = deposit(1, 88);
    pub const DepositFactor: Balance = deposit(0, 32);
}

impl pallet_multisig::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type Currency = Balances;
    type DepositBase = DepositBase;
    type DepositFactor = DepositFactor;
    type MaxSignatories = ConstU32<100>;
    type WeightInfo = pallet_multisig::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
    pub const Period: u32 = 6 * HOURS;
    pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = ConvertInto;
    type ShouldEndSession = ParachainStaking;
    type NextSessionRotation = ParachainStaking;
    type SessionManager = ParachainStaking;
    // Essentially just Aura, but lets be pedantic.
    type SessionHandler = <SessionKeys as sp_runtime::traits::OpaqueKeys>::KeyTypeIdProviders;
    type Keys = SessionKeys;
    type WeightInfo = ();
}

impl pallet_aura::Config for Runtime {
    type AuthorityId = AuraId;
    type DisabledValidators = ();
    type MaxAuthorities = ConstU32<100_000>;
    // Should be only enabled (`true`) when async backing is enabled
    // otherwise set to `false`
    type AllowMultipleBlocksPerSlot = ConstBool<true>;
    type SlotDuration = ConstU64<SLOT_DURATION>;
}

parameter_types! {
    pub const MaxCandidates: u32 = 1000;
    pub const MinEligibleCollators: u32 = 3;
    pub const SessionLength: BlockNumber = 6 * HOURS;
    pub const MaxInvulnerables: u32 = 100;
    pub const ExecutiveBody: BodyId = BodyId::Executive;
}

// Define the types required by the Scheduler pallet.
parameter_types! {
    pub MaximumSchedulerWeight: Weight = Perbill::from_percent(80) * MAXIMUM_BLOCK_WEIGHT;
    pub const MaxScheduledPerBlock: u32 = 50;
}

// Configure the runtime's implementation of the Scheduler pallet.
impl pallet_scheduler::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type RuntimeOrigin = RuntimeOrigin;
    type PalletsOrigin = OriginCaller;
    type RuntimeCall = RuntimeCall;
    type MaximumWeight = MaximumSchedulerWeight;
    type ScheduleOrigin = frame_system::EnsureRoot<AccountId>;
    type MaxScheduledPerBlock = MaxScheduledPerBlock;
    type WeightInfo = ();
    type Preimages = Preimage;
    type OriginPrivilegeCmp = EqualPrivilegeOnly;
}

parameter_types! {
    pub const MinVestedTransfer: Balance = 15 * OTP;
    pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
}

impl pallet_vesting::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type BlockNumberToBalance = ConvertInto;
    type MinVestedTransfer = MinVestedTransfer;
    type WeightInfo = pallet_vesting::weights::SubstrateWeight<Runtime>;
    type UnvestedFundsAllowedWithdrawReasons = UnvestedFundsAllowedWithdrawReasons;
    type BlockNumberProvider = System;
    // `VestingInfo` encode length is 36bytes. 28 schedules gets encoded as 1009 bytes, which is the
    // highest number of schedules that encodes less than 2^10.
    const MAX_VESTING_SCHEDULES: u32 = 28;
}

parameter_types! {
    pub const ProposalBond: Permill = Permill::from_percent(5);
    pub const ProposalBondMinimum: Balance = 100 * OTP;
    pub const ProposalBondMaximum: Balance = 500 * OTP;
    pub const SpendPeriod: BlockNumber = 24 * DAYS;
    pub const MaxApprovals: u32 = 100;
    pub TreasuryAccount: AccountId = Treasury::account_id();
}

impl pallet_treasury::Config for Runtime {
    type PalletId = TreasuryPalletId;
    type Currency = Balances;
    type ApproveOrigin = EnsureRoot<AccountId>;
    type RejectOrigin = EnsureRoot<AccountId>;
    type RuntimeEvent = RuntimeEvent;
    type OnSlash = Treasury;
    type ProposalBond = ProposalBond;
    type ProposalBondMinimum = ProposalBondMinimum;
    type ProposalBondMaximum = ProposalBondMaximum;
    type SpendPeriod = SpendPeriod;
    type Burn = ();
    type BurnDestination = ();
    type SpendFunds = ();
    type WeightInfo = pallet_treasury::weights::SubstrateWeight<Runtime>;
    type MaxApprovals = MaxApprovals;
    type SpendOrigin = frame_support::traits::NeverEnsureOrigin<u128>;

    type AssetKind = ();
	type Beneficiary = AccountId;
	type BeneficiaryLookup = IdentityLookup<AccountId>;
	type Paymaster = PayFromAccount<Balances, TreasuryAccount>;
	type BalanceConverter = UnityAssetBalanceConversion;
	type PayoutPeriod = ConstU32<0>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchmarkHelper;
}

pub struct MergeAccountEvm;
impl MergeAccount<AccountId> for MergeAccountEvm {
#[transactional]
fn merge_account(source: &AccountId, dest: &AccountId) -> DispatchResult {
     // unreserve all reserved currency
     <Balances as ReservableCurrency<_>>::unreserve(source, Balances::reserved_balance(source));

     // transfer all free to dest
     match Balances::transfer(&source, &dest, Balances::free_balance(source), ExistenceRequirement::AllowDeath,) {
       Ok(_) => Ok(()),
       Err(e) => Err(e),
     }
  }
}

impl pallet_evm_accounts::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
    type ChainId = ChainId;
	type AddressMapping = EvmAddressMapping<Runtime>;
	type MergeAccount = MergeAccountEvm;
	type WeightInfo = weights::evm_accounts_weights::WeightInfo<Runtime>;
}

parameter_types! {
	pub DefaultBaseFeePerGas: U256 = U256::from(16);
    pub DefaultElasticity: Permill = Permill::from_parts(125_000);
}

pub struct BaseFeeThreshold;
impl pallet_base_fee::BaseFeeThreshold for BaseFeeThreshold {
	fn lower() -> Permill {
		Permill::zero()
	}
	fn ideal() -> Permill {
		Permill::from_parts(500_000)
	}
	fn upper() -> Permill {
		Permill::from_parts(1_000_000)
	}
}

impl pallet_base_fee::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Threshold = BaseFeeThreshold;
	type DefaultBaseFeePerGas = DefaultBaseFeePerGas;
    	type DefaultElasticity = DefaultElasticity;
}

type FungibleAccountId<T> = <T as frame_system::Config>::AccountId;

type BalanceFor<T> = 
    <<T as pallet_evm::Config>::Currency as Inspect<FungibleAccountId<T>>>::Balance;

pub struct OnChargeEVMTransaction<OU>(sp_std::marker::PhantomData<OU>);
impl<T, OU> OnChargeEVMTransactionT<T> for OnChargeEVMTransaction<OU>
where
	T: pallet_evm::Config,
	T::Currency: Balanced<T::AccountId>,
    OU: OnUnbalanced<Credit<T::AccountId, T::Currency>>,
	U256: UniqueSaturatedInto<BalanceFor<T>>
{
	type LiquidityInfo = Option<Credit<T::AccountId, T::Currency>>;

	fn withdraw_fee(who: &H160, fee: U256) -> Result<Self::LiquidityInfo, pallet_evm::Error<T>> {
		EVMFungibleAdapter::<<T as pallet_evm::Config>::Currency, ()>::withdraw_fee(who, fee)
	}

	fn correct_and_deposit_fee(
		who: &H160,
		corrected_fee: U256,
		base_fee: U256,
		already_withdrawn: Self::LiquidityInfo,
	) -> Self::LiquidityInfo {
        <EVMFungibleAdapter<<T as pallet_evm::Config>::Currency, OU> as OnChargeEVMTransactionT<
            T,
		>>::correct_and_deposit_fee(who, corrected_fee, base_fee, already_withdrawn)
	}

	fn pay_priority_fee(tip: Self::LiquidityInfo) {
		if let Some(tip) = tip {
			OU::on_unbalanced(tip);
		}
	}
}

/// Current approximation of the gas/s consumption considering
/// EVM execution over compiled WASM (on 4.4Ghz CPU).
/// Given the 500ms Weight, from which 75% only are used for transactions,
/// the total EVM execution gas limit is: GAS_PER_SECOND * 0.500 * 0.75 ~= 15_000_000.
pub const GAS_PER_SECOND: u64 = 40_000_000;

/// Approximate ratio of the amount of Weight per Gas.
/// u64 works for approximations because Weight is a very small unit compared to gas.
pub const WEIGHT_PER_GAS: u64 = WEIGHT_REF_TIME_PER_SECOND.saturating_div(GAS_PER_SECOND);

pub struct FindAuthorTruncated<F>(sp_std::marker::PhantomData<F>);
impl<F: FindAuthor<u32>> FindAuthor<H160> for FindAuthorTruncated<F> {
    fn find_author<'a, I>(digests: I) -> Option<H160>
    where
        I: 'a + IntoIterator<Item = (ConsensusEngineId, &'a [u8])>,
    {
        if let Some(author_index) = F::find_author(digests) {
            let authority_id = pallet_aura::Authorities::<Runtime>::get()[author_index as usize].clone();
            return Some(H160::from_slice(&authority_id.encode()[4..24]));
        }

        None
    }
}

parameter_types! {
    pub const ChainId: u64 = 2043;
    pub BlockGasLimit: U256 = U256::from(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT.ref_time() / WEIGHT_PER_GAS);
    pub PrecompilesValue: FrontierPrecompiles<Runtime> = FrontierPrecompiles::<_>::new();
    pub WeightPerGas: Weight = Weight::from_parts(WEIGHT_PER_GAS, 0);
    /// The amount of gas per pov. A ratio of 4 if we convert ref_time to gas and we compare
	/// it with the pov_size for a block. E.g.
	/// ceil(
	///     (max_extrinsic.ref_time() / max_extrinsic.proof_size()) / WEIGHT_PER_GAS
	/// )
	pub const GasLimitPovSizeRatio: u64 = 4;
}

impl pallet_evm::Config for Runtime {
	type Currency = Balances;
	type RuntimeEvent = RuntimeEvent;

	type BlockGasLimit = BlockGasLimit;
	type ChainId = ChainId;
	type BlockHashMapping = EthereumBlockHashMapping<Self>;
	type Runner = pallet_evm::runner::stack::Runner<Self>;

    type CallOrigin = EnsureAddressRoot<AccountId>;
	type WithdrawOrigin = EnsureAddressNever<AccountId>;
	type AddressMapping = EvmAddressMapping<Runtime>;

	type FeeCalculator = BaseFee;
	type GasWeightMapping = pallet_evm::FixedGasWeightMapping<Self>;
    type WeightPerGas = WeightPerGas;
	type OnChargeTransaction = OnChargeEVMTransaction<DealWithFees>;
    type OnCreate = ();
	type FindAuthor = FindAuthorTruncated<Aura>;
	type PrecompilesType = FrontierPrecompiles<Self>;
	type PrecompilesValue = PrecompilesValue;
    type GasLimitPovSizeRatio = GasLimitPovSizeRatio;
    type SuicideQuickClearLimit = ConstU32<0>;
    type Timestamp = Timestamp;
    type WeightInfo = pallet_evm::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const PostBlockAndTxnHashes: PostLogContent = PostLogContent::BlockAndTxnHashes;
}

impl pallet_ethereum::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type StateRoot = pallet_ethereum::IntermediateStateRoot<Self>;
    type PostLogContent = PostBlockAndTxnHashes;
    type ExtraDataLength = ConstU32<30>;
}

/// Id used for identifying assets.
///
/// AssetId allocation:
/// [1; 2^32-1]     Custom user assets (permissionless)
/// [2^32; 2^64-1]  Statemine assets (simple map)
/// [2^64; 2^128-1] Ecosystem assets
/// 2^128-1         Relay chain token (DOT)
pub type AssetId = u128;

impl AddressToAssetId<AssetId> for Runtime {
    fn address_to_asset_id(address: H160) -> Option<AssetId> {
        let mut data = [0u8; 16];
        let address_bytes: [u8; 20] = address.into();
        if ASSET_PRECOMPILE_ADDRESS_PREFIX.eq(&address_bytes[0..4]) {
            data.copy_from_slice(&address_bytes[4..20]);
            Some(u128::from_be_bytes(data))
        } else {
            None
        }
    }

    fn asset_id_to_address(asset_id: AssetId) -> H160 {
        let mut data = [0u8; 20];
        data[0..4].copy_from_slice(ASSET_PRECOMPILE_ADDRESS_PREFIX);
        data[4..20].copy_from_slice(&asset_id.to_be_bytes());
        H160::from(data)
    }
}

parameter_types! {
	pub const AssetDeposit: Balance = 100 * OTP;
	pub const ApprovalDeposit: Balance = 0;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10 * OTP;
	pub const MetadataDepositPerByte: Balance = 1 * OTP;
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = AssetId;
    type AssetIdParameter = codec::Compact<u128>;
	type Currency = Balances;
    type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<AccountId>>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<OTP>;
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

pub struct EvmRevertCodeHandler;
impl pallet_xc_asset_config::XcAssetChanged<Runtime> for EvmRevertCodeHandler {
    fn xc_asset_registered(asset_id: AssetId) {
        let address = Runtime::asset_id_to_address(asset_id);
        pallet_evm::AccountCodes::<Runtime>::insert(address, vec![0x60, 0x00, 0x60, 0x00, 0xfd]);
    }

    fn xc_asset_unregistered(asset_id: AssetId) {
        let address = Runtime::asset_id_to_address(asset_id);
        pallet_evm::AccountCodes::<Runtime>::remove(address);
    }
}

impl pallet_xc_asset_config::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type AssetId = AssetId;
    type XcAssetChanged = EvmRevertCodeHandler;
    type ManagerOrigin = frame_system::EnsureRoot<AccountId>;
    type WeightInfo = weights::pallet_xc_asset_config::WeightInfo<Self>;
}

parameter_types! {
	pub const PreimageMaxSize: u32 = 4096 * 1024;
	pub const PreimageBaseDeposit: Balance = deposit(2, 64);
	pub const PreimageByteDeposit: Balance = 1 * MILLIOTP;
    pub const PreimageHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::Preimage(pallet_preimage::HoldReason::Preimage);
}

impl pallet_preimage::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type ManagerOrigin = EnsureRoot<AccountId>;
    type WeightInfo = pallet_preimage::weights::SubstrateWeight<Runtime>;
    type Consideration = HoldConsideration<
		AccountId,
		Balances,
		PreimageHoldReason,
		LinearStoragePrice<PreimageBaseDeposit, PreimageByteDeposit, Balance>,
	>;
}

impl pallet_utility::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = pallet_utility::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const CouncilMotionDuration: BlockNumber = 3 * DAYS;
	pub const CouncilMaxProposals: u32 = 100;
	pub const CouncilMaxMembers: u32 = 5;
    pub MaxCollectivesProposalWeight: Weight = Perbill::from_percent(50) * RuntimeBlockWeights::get().max_block;
}

type CouncilCollective = pallet_collective::Instance1;
impl pallet_collective::Config<CouncilCollective> for Runtime {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = CouncilMotionDuration;
	type MaxProposals = CouncilMaxProposals;
	type MaxMembers = CouncilMaxMembers;
	type DefaultVote = pallet_collective::MoreThanMajorityThenPrimeDefaultVote;
    type SetMembersOrigin = EnsureRoot<Self::AccountId>;
    type WeightInfo = pallet_collective::weights::SubstrateWeight<Runtime>;
    type MaxProposalWeight = MaxCollectivesProposalWeight;
}

parameter_types! {
	pub const LaunchPeriod: BlockNumber = 5 * DAYS;
	pub const VotingPeriod: BlockNumber = 5 * DAYS;
    pub const VotingLockingPeriod: BlockNumber = 1 * DAYS;
	pub const FastTrackVotingPeriod: BlockNumber = 3 * HOURS;
	pub const MinimumDeposit: Balance = 1000 * OTP;
	pub const EnactmentPeriod: BlockNumber = 2 * DAYS;
	pub const CooloffPeriod: BlockNumber = 7 * DAYS;
	pub const MaxProposals: u32 = 100;
}

type EnsureRootOrTwoFiftsOfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 2, 5>,
>;
type EnsureRootOrThreeFiftsOfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 3, 5>,
>;
type EnsureRootOrFourFiftsOfCouncil = EitherOfDiverse<
	EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, CouncilCollective, 4, 5>,
>;

impl pallet_democracy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type EnactmentPeriod = EnactmentPeriod;
	type LaunchPeriod = LaunchPeriod;
	type VotingPeriod = VotingPeriod;
	type VoteLockingPeriod = VotingLockingPeriod; // Same as EnactmentPeriod
	type MinimumDeposit = MinimumDeposit;
	/// A straight majority of the council can decide what their next motion is.
	type ExternalOrigin = EnsureRootOrTwoFiftsOfCouncil;
	/// A super-majority can have the next scheduled referendum be a straight majority-carries vote.
	type ExternalMajorityOrigin = EnsureRootOrThreeFiftsOfCouncil;
	/// A unanimous council can have the next scheduled referendum be a straight default-carries
	/// (NTB) vote.
	type ExternalDefaultOrigin = EnsureRootOrThreeFiftsOfCouncil;
	type SubmitOrigin = EnsureSigned<AccountId>;
	/// Two thirds of the technical committee can have an ExternalMajority/ExternalDefault vote
	/// be tabled immediately and with a shorter voting/enactment period.
	type FastTrackOrigin = EnsureRootOrThreeFiftsOfCouncil;
	type InstantOrigin = EnsureRootOrFourFiftsOfCouncil;
	type InstantAllowed = frame_support::traits::ConstBool<true>;
	type FastTrackVotingPeriod = FastTrackVotingPeriod;
	// To cancel a proposal which has been passed, 3/5 of the council must agree to it.
	type CancellationOrigin = EnsureRootOrThreeFiftsOfCouncil;
	// To cancel a proposal before it has been passed, the technical committee must be unanimous or
	// Root must agree.
	type CancelProposalOrigin = EnsureRootOrThreeFiftsOfCouncil;
	type BlacklistOrigin = EnsureRoot<AccountId>;
	// Any single technical committee member may veto a coming council proposal, however they can
	// only do it once and it lasts only for the cool-off period.
	type VetoOrigin = pallet_collective::EnsureMember<AccountId, CouncilCollective>;
	type CooloffPeriod = CooloffPeriod;
	type Slash = Treasury;
	type Scheduler = Scheduler;
	type PalletsOrigin = OriginCaller;
	type MaxVotes = ConstU32<100>;
	type MaxProposals = MaxProposals;
	type Preimages = Preimage;
	type MaxDeposits = ConstU32<100>;
	type MaxBlacklisted = ConstU32<100>;
    type WeightInfo = pallet_democracy::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	pub const BasicDeposit: Balance = deposit(1, 258);      // 258 bytes on-chain
	pub const ByteDeposit: Balance = deposit(0, 1);
	pub const SubAccountDeposit: Balance = deposit(1, 53);   // 53 bytes on-chain
	pub const MaxSubAccounts: u32 = 100;
	pub const MaxAdditionalFields: u32 = 100;
	pub const MaxRegistrars: u32 = 20;
}

impl pallet_identity::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = BasicDeposit;
	type ByteDeposit = ByteDeposit;
	type SubAccountDeposit = SubAccountDeposit;
	type MaxSubAccounts = MaxSubAccounts;
    type IdentityInformation = IdentityInfo<MaxAdditionalFields>;
	type MaxRegistrars = MaxRegistrars;
	type Slashed = Treasury;
	type ForceOrigin = EnsureRootOrThreeFiftsOfCouncil;
	type RegistrarOrigin = EnsureRootOrThreeFiftsOfCouncil;
    type OffchainSignature = Signature;
    type SigningPublicKey = <Signature as sp_runtime::traits::Verify>::Signer;
    type UsernameAuthorityOrigin = EnsureRootOrThreeFiftsOfCouncil;
    type PendingUsernameExpiration = ConstU32<{ 7 * DAYS }>;
    type MaxSuffixLength = ConstU32<7>;
    type MaxUsernameLength = ConstU32<32>;
	type WeightInfo = pallet_identity::weights::SubstrateWeight<Runtime>;
}

parameter_types! {
	// One storage item; key size 32, value size 8; .
	pub const ProxyDepositBase: Balance = deposit(1, 8);
	// Additional storage item size of 33 bytes.
	pub const ProxyDepositFactor: Balance = deposit(0, 33);
	pub const AnnouncementDepositBase: Balance = deposit(1, 8);
	pub const AnnouncementDepositFactor: Balance = deposit(0, 66);
}

/// The type used to represent the kinds of proxying allowed.
#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	Any,
	NonTransfer,
	Governance,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::NonTransfer => !matches!(
				c,
				RuntimeCall::Balances(..) |
					RuntimeCall::Assets(..) |
					RuntimeCall::Vesting(pallet_vesting::Call::vested_transfer { .. })
			),
			ProxyType::Governance => matches!(
				c,
				RuntimeCall::Democracy(..) |
					RuntimeCall::Council(..) |
					RuntimeCall::Treasury(..)
			),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		match (self, o) {
			(x, y) if x == y => true,
			(ProxyType::Any, _) => true,
			(_, ProxyType::Any) => false,
			(ProxyType::NonTransfer, _) => true,
			_ => false,
		}
	}
}

impl pallet_proxy::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ProxyDepositBase;
	type ProxyDepositFactor = ProxyDepositFactor;
	type MaxProxies = ConstU32<32>;
	type WeightInfo = pallet_proxy::weights::SubstrateWeight<Runtime>;
	type MaxPending = ConstU32<32>;
	type CallHasher = BlakeTwo256;
	type AnnouncementDepositBase = AnnouncementDepositBase;
	type AnnouncementDepositFactor = AnnouncementDepositFactor;
}

parameter_types! {
	/// Default fixed percent a collator takes off the top of due rewards
	pub const DefaultCollatorCommission: Perbill = Perbill::from_percent(33);
	/// Default percent of inflation set aside for parachain bond every round
	pub const DefaultParachainBondReservePercent: Percent = Percent::from_percent(0);
	pub const MinDelegation: Balance = 100 * OTP;
	pub const MinDelegatorStk: Balance = 100 * OTP;
	pub const MinCandidateStk: Balance = 100000 * OTP;
}

ord_parameter_types! {
	pub const StakingPot: AccountId =
		AccountIdConversion::<AccountId>::into_account_truncating(&CollatorsIncentivesPalletId::get());
}


impl pallet_parachain_staking::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
    type RuntimeHoldReason = RuntimeHoldReason;
	type PayMaster = StakingPot;
    type Balance = Balance;
	type Currency = Balances;
	type MonetaryGovernanceOrigin = EnsureRootOrFourFiftsOfCouncil;
	/// Minimum round length is 2 minutes (10 * 6 second block times)
	type MinBlocksPerRound = ConstU32<{3 * HOURS}>;
	/// Rounds before the collator leaving the candidates request can be executed
	type LeaveCandidatesDelay = ConstU32<{ 4 * 7 }>;
	/// Rounds before the candidate bond increase/decrease can be executed
	type CandidateBondLessDelay = ConstU32<{ 4 * 7 }>;
	/// Rounds before the delegator exit can be executed
	type LeaveDelegatorsDelay = ConstU32<{ 4 * 7 }>;
	/// Rounds before the delegator revocation can be executed
	type RevokeDelegationDelay = ConstU32<{ 4 * 7 }>;
	/// Rounds before the delegator bond increase/decrease can be executed
	type DelegationBondLessDelay = ConstU32<{ 4 * 7 }>;
	/// Rounds before the reward is paid
	type RewardPaymentDelay = ConstU32<2>;
	/// Minimum collators selected per round, default at genesis and minimum forever after
    /// TODO: Incerease this after release
	type MinSelectedCandidates = ConstU32<1>;
	/// Maximum top delegations per candidate
	type MaxTopDelegationsPerCandidate = ConstU32<300>;
	/// Maximum bottom delegations per candidate
	type MaxBottomDelegationsPerCandidate = ConstU32<50>;
	/// Maximum delegations per delegator
	type MaxDelegationsPerDelegator = ConstU32<100>;
	/// Minimum stake required to be reserved to be a candidate
	type MinCandidateStk = MinCandidateStk;
	/// Minimum stake for any registered on-chain account to delegate
	type MinDelegation = MinDelegation;
    /// Minimum stake for any registered on-chain account to be a delegator
    type MinDelegatorStk = MinDelegatorStk;
	// We use the default implementation, so we leave () here.
    type OnCollatorPayout = ();
   	// We use the default implementation, so we leave () here.
	type PayoutCollatorReward = ();
    // We use the default implementation, so we leave () here.
	type OnNewRound = ();
	type WeightInfo = weights::pallet_parachain_staking::WeightInfo<Runtime>;
}


// Create the runtime by composing the FRAME pallets that were previously configured.
construct_runtime!(
    pub struct Runtime
    {
        // System support stuff.
        System: frame_system::{Pallet, Call, Config<T>, Storage, Event<T>} = 0,
        ParachainSystem: cumulus_pallet_parachain_system::{
            Pallet, Call, Config<T>, Storage, Inherent, Event<T>, ValidateUnsigned,
        } = 1,
        Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent} = 2,
        ParachainInfo: parachain_info::{Pallet, Storage, Config<T>} = 3,
        Utility: pallet_utility = 4,
        Multisig: pallet_multisig = 5,
        Proxy: pallet_proxy = 6,

        // Monetary stuff.
        Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>} = 10,
        TransactionPayment: pallet_transaction_payment::{Pallet, Storage, Config<T>, Event<T>} = 11,
        Vesting: pallet_vesting::{Pallet, Call, Storage, Event<T>, Config<T>} = 12,
        Treasury: pallet_treasury::{Pallet, Call, Storage, Config<T>, Event<T>} = 13,
        Assets: pallet_assets::{Pallet, Call, Storage, Event<T>} = 14,
        XcAssetConfig: pallet_xc_asset_config = 15,

        // Collator support. The order of these 4 are important and shall not change.
        Authorship: pallet_authorship::{Pallet, Storage} = 20,
        // Disebled after implementation of Staking
        // CollatorSelection: pallet_collator_selection::{Pallet, Call, Storage, Event<T>, Config<T>} = 21,
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>} = 22,
        Aura: pallet_aura::{Pallet, Storage, Config<T>} = 23,
        AuraExt: cumulus_pallet_aura_ext::{Pallet, Storage, Config<T>} = 24,
        ParachainStaking: pallet_parachain_staking = 25,

        // XCM helpers.
        XcmpQueue: cumulus_pallet_xcmp_queue::{Pallet, Call, Storage, Event<T>} = 30,
        PolkadotXcm: pallet_xcm::{Pallet, Call, Event<T>, Origin, Storage, Config<T>} = 31,
        CumulusXcm: cumulus_pallet_xcm::{Pallet, Event<T>, Origin} = 32,
        DmpQueue: cumulus_pallet_dmp_queue::{Pallet, Call, Storage, Event<T>} = 33,
        MessageQueue: pallet_message_queue = 34,

        // Frontier
		EVM: pallet_evm::{Pallet, Config<T>, Call, Storage, Event<T>} = 50,
        Ethereum: pallet_ethereum::{Pallet, Call, Storage, Event, Config<T>, Origin} = 51,
        EvmAccounts: pallet_evm_accounts::{Pallet, Call, Storage, Event<T>} = 52,
        BaseFee: pallet_base_fee::{Pallet, Call, Storage, Config<T>, Event} = 53,

        // Governance
        Scheduler: pallet_scheduler::{Pallet, Storage, Event<T>, Call} = 60,
        Preimage: pallet_preimage = 61,
        Council: pallet_collective::<Instance1> = 62,
        Democracy: pallet_democracy = 63,
        Identity: pallet_identity = 64,
    }
);

#[cfg(feature = "runtime-benchmarks")]
#[macro_use]
extern crate frame_benchmarking;

#[cfg(feature = "runtime-benchmarks")]
mod benches {
    frame_benchmarking::define_benchmarks!(
        [frame_system, SystemBench::<Runtime>]
        [pallet_balances, Balances]
        [pallet_collective, CouncilCollective]
        [pallet_democracy, Democracy]
        [pallet_identity, Identity]
        [pallet_preimage, Preimage]
        [pallet_proxy, Proxy]
        [pallet_timestamp, Timestamp]
        [pallet_parachain_staking, ParachainStaking]
        [cumulus_pallet_xcmp_queue, XcmpQueue]
        [pallet_utility, Utility]
    );
}


impl fp_self_contained::SelfContainedCall for RuntimeCall {
	type SignedInfo = H160;

	fn is_self_contained(&self) -> bool {
		match self {
			RuntimeCall::Ethereum(call) => call.is_self_contained(),
			_ => false,
		}
	}

	fn check_self_contained(&self) -> Option<Result<Self::SignedInfo, TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.check_self_contained(),
			_ => None,
		}
	}

	fn validate_self_contained(
		&self,
		info: &Self::SignedInfo,
		dispatch_info: &DispatchInfoOf<RuntimeCall>,
		len: usize,
	) -> Option<TransactionValidity> {
		match self {
			RuntimeCall::Ethereum(call) => call.validate_self_contained(info, dispatch_info, len),
			_ => None,
		}
	}

	fn pre_dispatch_self_contained(
		&self,
		info: &Self::SignedInfo,
        dispatch_info: &DispatchInfoOf<RuntimeCall>,
        len: usize,
	) -> Option<Result<(), TransactionValidityError>> {
		match self {
			RuntimeCall::Ethereum(call) => call.pre_dispatch_self_contained(info, dispatch_info, len),
			_ => None,
		}
	}

	fn apply_self_contained(
		self,
		info: Self::SignedInfo,
	) -> Option<sp_runtime::DispatchResultWithInfo<PostDispatchInfoOf<Self>>> {
		match self {
			call @ RuntimeCall::Ethereum(pallet_ethereum::Call::transact { .. }) => Some(call.dispatch(
				RuntimeOrigin::from(pallet_ethereum::RawOrigin::EthereumTransaction(info)),
			)),
			_ => None,
		}
	}
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Balance of an account.
pub type Balance = u128;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// An index to a block.
pub type BlockNumber = u32;

/// The address format for describing accounts.
pub type Address = AccountId;

/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;

// /// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;

/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;

/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;

/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = (
    frame_system::CheckNonZeroSender<Runtime>,
    frame_system::CheckSpecVersion<Runtime>,
    frame_system::CheckTxVersion<Runtime>,
    frame_system::CheckGenesis<Runtime>,
    frame_system::CheckEra<Runtime>,
    frame_system::CheckNonce<Runtime>,
    frame_system::CheckWeight<Runtime>,
    pallet_transaction_payment::ChargeTransactionPayment<Runtime>,
    cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim<Runtime>,
    frame_metadata_hash_extension::CheckMetadataHash<Runtime>,
);

/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = fp_self_contained::UncheckedExtrinsic<Address, RuntimeCall, Signature, SignedExtra>;

/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, RuntimeCall, SignedExtra>;

/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
    Runtime,
    Block,
    frame_system::ChainContext<Runtime>,
    Runtime,
    AllPalletsWithSystem,
>;

impl_runtime_apis! {
    impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
        fn slot_duration() -> sp_consensus_aura::SlotDuration {
            sp_consensus_aura::SlotDuration::from_millis(SLOT_DURATION)
        }

        fn authorities() -> Vec<AuraId> {
            pallet_aura::Authorities::<Runtime>::get().into_inner()
        }
    }

    impl cumulus_primitives_aura::AuraUnincludedSegmentApi<Block> for Runtime {
        fn can_build_upon(
            included_hash: <Block as BlockT>::Hash,
            slot: cumulus_primitives_aura::Slot,
        ) -> bool {
            ConsensusHook::can_build_upon(included_hash, slot)
        }
    }

    impl sp_api::Core<Block> for Runtime {
        fn version() -> RuntimeVersion {
            VERSION
        }

        fn execute_block(block: Block) {
            Executive::execute_block(block)
        }

        fn initialize_block(header: &<Block as BlockT>::Header) -> sp_runtime::ExtrinsicInclusionMode {
            Executive::initialize_block(header)
        }
    }

    impl sp_api::Metadata<Block> for Runtime {
        fn metadata() -> OpaqueMetadata {
            OpaqueMetadata::new(Runtime::metadata().into())
        }

        fn metadata_at_version(version: u32) -> Option<OpaqueMetadata> {
			Runtime::metadata_at_version(version)
		}

		fn metadata_versions() -> Vec<u32> {
			Runtime::metadata_versions()
		}
    }

    impl sp_block_builder::BlockBuilder<Block> for Runtime {
        fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
            Executive::apply_extrinsic(extrinsic)
        }

        fn finalize_block() -> <Block as BlockT>::Header {
            Executive::finalize_block()
        }

        fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
            data.create_extrinsics()
        }

        fn check_inherents(
            block: Block,
            data: sp_inherents::InherentData,
        ) -> sp_inherents::CheckInherentsResult {
            data.check_extrinsics(&block)
        }
    }

    impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
        fn validate_transaction(
            source: TransactionSource,
            tx: <Block as BlockT>::Extrinsic,
            block_hash: <Block as BlockT>::Hash,
        ) -> TransactionValidity {
            Executive::validate_transaction(source, tx, block_hash)
        }
    }

    impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
        fn offchain_worker(header: &<Block as BlockT>::Header) {
            Executive::offchain_worker(header)
        }
    }

    impl fp_rpc::EthereumRuntimeRPCApi<Block> for Runtime {
		fn chain_id() -> u64 {
			<Runtime as pallet_evm::Config>::ChainId::get()
		}

		fn account_basic(address: H160) -> EVMAccount {
			let (account, _) = EVM::account_basic(&address);
			account
		}

		fn gas_price() -> U256 {
			let (gas_price, _) = <Runtime as pallet_evm::Config>::FeeCalculator::min_gas_price();
			gas_price
		}

		fn account_code_at(address: H160) -> Vec<u8> {
			pallet_evm::AccountCodes::<Runtime>::get(address)
		}

		fn author() -> H160 {
			<pallet_evm::Pallet<Runtime>>::find_author()
		}

		fn storage_at(address: H160, index: U256) -> H256 {
			let mut tmp = [0u8; 32];
			index.to_big_endian(&mut tmp);
            pallet_evm::AccountStorages::<Runtime>::get(address, H256::from_slice(&tmp[..]))
		}

		fn call(
			from: H160,
			to: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CallInfo, sp_runtime::DispatchError> {
            use pallet_evm::GasWeightMapping as _;

			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

            let is_transactional = false;
            let validate = true;

            // Estimated encoded transaction size must be based on the heaviest transaction
            // type (EIP1559Transaction) to be compatible with all transaction types.
            let mut estimated_transaction_len = data.len() +
                // pallet ethereum index: 1
                // transact call index: 1
                // Transaction enum variant: 1
                // chain_id 8 bytes
                // nonce: 32
                // max_priority_fee_per_gas: 32
                // max_fee_per_gas: 32
                // gas_limit: 32
                // action: 21 (enum varianrt + call address)
                // value: 32
                // access_list: 1 (empty vec size)
                // 65 bytes signature
                258;

            if access_list.is_some() {
                estimated_transaction_len += access_list.encoded_size();
            }


            let gas_limit = if gas_limit > U256::from(u64::MAX) {
                u64::MAX
            } else {
                gas_limit.low_u64()
            };
			let without_base_extrinsic_weight = true;

			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
					gas_limit,
					without_base_extrinsic_weight
				) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None),
				};

			<Runtime as pallet_evm::Config>::Runner::call(
				from,
				to,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
				is_transactional,
				validate,
				weight_limit,
				proof_size_base_cost,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
			).map_err(|err| err.error.into())
		}

		fn create(
			from: H160,
			data: Vec<u8>,
			value: U256,
			gas_limit: U256,
			max_fee_per_gas: Option<U256>,
			max_priority_fee_per_gas: Option<U256>,
			nonce: Option<U256>,
			estimate: bool,
			access_list: Option<Vec<(H160, Vec<H256>)>>,
		) -> Result<pallet_evm::CreateInfo, sp_runtime::DispatchError> {
			use pallet_evm::GasWeightMapping as _;

			let config = if estimate {
				let mut config = <Runtime as pallet_evm::Config>::config().clone();
				config.estimate = true;
				Some(config)
			} else {
				None
			};

            let is_transactional = false;
            let validate = true;

			let mut estimated_transaction_len = data.len() +
				// from: 20
				// value: 32
				// gas_limit: 32
				// nonce: 32
				// 1 byte transaction action variant
				// chain id 8 bytes
				// 65 bytes signature
				190;

			if max_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if max_priority_fee_per_gas.is_some() {
				estimated_transaction_len += 32;
			}
			if access_list.is_some() {
				estimated_transaction_len += access_list.encoded_size();
			}


			let gas_limit = if gas_limit > U256::from(u64::MAX) {
				u64::MAX
			} else {
				gas_limit.low_u64()
			};
			let without_base_extrinsic_weight = true;

			let (weight_limit, proof_size_base_cost) =
				match <Runtime as pallet_evm::Config>::GasWeightMapping::gas_to_weight(
					gas_limit,
					without_base_extrinsic_weight
				) {
					weight_limit if weight_limit.proof_size() > 0 => {
						(Some(weight_limit), Some(estimated_transaction_len as u64))
					}
					_ => (None, None),
				};

			<Runtime as pallet_evm::Config>::Runner::create(
				from,
				data,
				value,
				gas_limit.unique_saturated_into(),
				max_fee_per_gas,
				max_priority_fee_per_gas,
				nonce,
				access_list.unwrap_or_default(),
                is_transactional,
                validate,
				weight_limit,
				proof_size_base_cost,
				config.as_ref().unwrap_or(<Runtime as pallet_evm::Config>::config()),
			).map_err(|err| err.error.into())
		}

		fn current_transaction_statuses() -> Option<Vec<TransactionStatus>> {
			pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
		}

		fn current_block() -> Option<pallet_ethereum::Block> {
			pallet_ethereum::CurrentBlock::<Runtime>::get()
		}

		fn current_receipts() -> Option<Vec<pallet_ethereum::Receipt>> {
			pallet_ethereum::CurrentReceipts::<Runtime>::get()
		}

		fn current_all() -> (
			Option<pallet_ethereum::Block>,
			Option<Vec<pallet_ethereum::Receipt>>,
			Option<Vec<TransactionStatus>>
		) {
			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentReceipts::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
			)
		}

		fn extrinsic_filter(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> Vec<EthereumTransaction> {
			xts.into_iter().filter_map(|xt| match xt.0.function {
				RuntimeCall::Ethereum(transact { transaction }) => Some(transaction),
				_ => None
			}).collect::<Vec<EthereumTransaction>>()
		}

		fn elasticity() -> Option<Permill> {
            Some(pallet_base_fee::Elasticity::<Runtime>::get())
		}

        fn gas_limit_multiplier_support() {}

        fn pending_block(
			xts: Vec<<Block as BlockT>::Extrinsic>,
		) -> (Option<pallet_ethereum::Block>, Option<Vec<TransactionStatus>>) {
			for ext in xts.into_iter() {
				let _ = Executive::apply_extrinsic(ext);
			}

			Ethereum::on_finalize(System::block_number() + 1);

			(
				pallet_ethereum::CurrentBlock::<Runtime>::get(),
				pallet_ethereum::CurrentTransactionStatuses::<Runtime>::get()
			)
		}
	}

    impl fp_rpc::ConvertTransactionRuntimeApi<Block> for Runtime {
		fn convert_transaction(transaction: EthereumTransaction) -> <Block as BlockT>::Extrinsic {
			UncheckedExtrinsic::new_unsigned(
				pallet_ethereum::Call::<Runtime>::transact { transaction }.into(),
			)
		}
	}

    impl sp_session::SessionKeys<Block> for Runtime {
        fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
            SessionKeys::generate(seed)
        }

        fn decode_session_keys(
            encoded: Vec<u8>,
        ) -> Option<Vec<(Vec<u8>, KeyTypeId)>> {
            SessionKeys::decode_into_raw_public_keys(&encoded)
        }
    }

    impl frame_system_rpc_runtime_api::AccountNonceApi<Block, AccountId, Nonce> for Runtime {
        fn account_nonce(account: AccountId) -> Nonce {
            System::account_nonce(account)
        }
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentApi<Block, Balance> for Runtime {
        fn query_info(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment_rpc_runtime_api::RuntimeDispatchInfo<Balance> {
            TransactionPayment::query_info(uxt, len)
        }
        fn query_fee_details(
            uxt: <Block as BlockT>::Extrinsic,
            len: u32,
        ) -> pallet_transaction_payment::FeeDetails<Balance> {
            TransactionPayment::query_fee_details(uxt, len)
        }
        fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
    }

    impl pallet_transaction_payment_rpc_runtime_api::TransactionPaymentCallApi<Block, Balance, RuntimeCall>
		for Runtime
	{
		fn query_call_info(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::RuntimeDispatchInfo<Balance> {
			TransactionPayment::query_call_info(call, len)
		}
		fn query_call_fee_details(
			call: RuntimeCall,
			len: u32,
		) -> pallet_transaction_payment::FeeDetails<Balance> {
			TransactionPayment::query_call_fee_details(call, len)
		}
        fn query_weight_to_fee(weight: Weight) -> Balance {
			TransactionPayment::weight_to_fee(weight)
		}
		fn query_length_to_fee(length: u32) -> Balance {
			TransactionPayment::length_to_fee(length)
		}
	}

    impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
        fn collect_collation_info(header: &<Block as BlockT>::Header) -> cumulus_primitives_core::CollationInfo {
            ParachainSystem::collect_collation_info(header)
        }
    }

    #[cfg(feature = "try-runtime")]
    impl frame_try_runtime::TryRuntime<Block> for Runtime {
        fn on_runtime_upgrade(checks: frame_try_runtime::UpgradeCheckSelect) -> (Weight, Weight) {
			let weight = Executive::try_runtime_upgrade(checks).unwrap();
            (weight, RuntimeBlockWeights::get().max_block)
        }

        fn execute_block(
			block: Block,
			state_root_check: bool,
			signature_check: bool,
			select: frame_try_runtime::TryStateSelect,
		) -> Weight {
			// NOTE: intentional unwrap: we don't want to propagate the error backwards, and want to
			// have a backtrace here.
			Executive::try_execute_block(block, state_root_check, signature_check, select).unwrap()
		}
    }

    impl sp_genesis_builder::GenesisBuilder<Block> for Runtime {
        fn build_state(config: Vec<u8>) -> sp_genesis_builder::Result {
            build_state::<RuntimeGenesisConfig>(config)
        }

        fn get_preset(id: &Option<sp_genesis_builder::PresetId>) -> Option<Vec<u8>> {
            get_preset::<RuntimeGenesisConfig>(id, |_| None)
        }
      
        fn preset_names() -> Vec<sp_genesis_builder::PresetId> {
            vec![]
        }
    }

    #[cfg(feature = "runtime-benchmarks")]
    impl frame_benchmarking::Benchmark<Block> for Runtime {
        fn benchmark_metadata(extra: bool) -> (
            Vec<frame_benchmarking::BenchmarkList>,
            Vec<frame_support::traits::StorageInfo>,
        ) {
            use frame_benchmarking::{Benchmarking, BenchmarkList};
            use frame_support::traits::StorageInfoTrait;
            use frame_system_benchmarking::Pallet as SystemBench;

            let mut list = Vec::<BenchmarkList>::new();
            list_benchmarks!(list, extra);

            let storage_info = AllPalletsWithSystem::storage_info();
            return (list, storage_info)
        }

        fn dispatch_benchmark(
            config: frame_benchmarking::BenchmarkConfig
        ) -> Result<Vec<frame_benchmarking::BenchmarkBatch>, sp_runtime::RuntimeString> {
            use frame_benchmarking::{Benchmarking, BenchmarkBatch};

            use frame_system_benchmarking::Pallet as SystemBench;
            impl frame_system_benchmarking::Config for Runtime {}

            use frame_support::traits::{TrackedStorageKey, WhitelistedStorageKeys};
			let whitelist = AllPalletsWithSystem::whitelisted_storage_keys();

            let mut batches = Vec::<BenchmarkBatch>::new();
            let params = (&config, &whitelist);
            add_benchmarks!(params, batches);

            if batches.is_empty() { return Err("Benchmark not found for this pallet.".into()) }
            Ok(batches)
        }
    }
}

cumulus_pallet_parachain_system::register_validate_block! {
    Runtime = Runtime,
    BlockExecutor = cumulus_pallet_aura_ext::BlockExecutor::<Runtime, Executive>,
}
