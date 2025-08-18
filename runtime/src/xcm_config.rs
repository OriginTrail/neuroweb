use super::{
    AccountId, AllPalletsWithSystem, Balance, Balances, DealWithFees, ForeignAssets, ParachainInfo,
    ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, WeightToFee,
    XcmpQueue, MILLIOTP, UNITS,
};
use crate::assets::EXISTENTIAL_DEPOSIT;
use codec::Encode;
use core::marker::PhantomData;
use frame_support::{
    parameter_types,
    traits::{ConstU32, Contains, Everything, Get, Nothing, PalletInfoAccess},
    weights::Weight,
};
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use scale_info::prelude::vec;
use sp_core::blake2_256;
use xcm::latest::prelude::*;
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, EnsureXcmOrigin, FixedWeightBounds, FungibleAdapter,
    FungiblesAdapter, IsConcrete, NativeAsset, NoChecking, ParentIsPreset, RelayChainAsNative,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, UsingComponents,
    WithComputedOrigin,
};
use xcm_executor::{
    traits::{ConvertLocation, WithOriginFilter},
    XcmExecutor,
};

parameter_types! {
    pub const RelayLocation: Location = Location::parent();
    pub const RelayNetwork: NetworkId = NetworkId::Polkadot;
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();

    pub TokenLocation: Location = Location {
        parents:0,
        interior: [
            PalletInstance(<Balances as PalletInfoAccess>::index() as u8)
        ].into()
    };

    pub UniversalLocation: InteriorLocation = [GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into())].into();
    pub CheckingAccount: AccountId = PolkadotXcm::check_account();
}

/// Type for specifying how a `Location` can be converted into an `AccountId`. This is used
/// when determining ownership of accounts for asset transacting and when attempting to use XCM
/// `Transact` in order to determine the dispatch Origin.
pub type LocationToAccountId = (
    // The parent (Relay-chain) origin converts to the parent `AccountId`.
    ParentIsPreset<AccountId>,
    // Sibling parachain origins convert to AccountId via the `ParaId::into`.
    SiblingParachainConvertsVia<Sibling, AccountId>,
    // Straight up local `AccountId32` origins just alias directly to `AccountId`.
    AccountId32Aliases<RelayNetwork, AccountId>,
    // Ethereum contract sovereign account.
    // (Used to get convert ethereum contract locations to sovereign account)
    ExternalConsensusLocationsConverterFor<UniversalLocation, AccountId>,
);

// Handle native currency (NEURO) via Balances pallet
pub type NativeAssetTransactor =
    FungibleAdapter<Balances, IsConcrete<TokenLocation>, LocationToAccountId, AccountId, ()>;

pub type ForeignAssetTransactor = FungiblesAdapter<
    ForeignAssets,
    ForeignAssetsConvertedConcreteId,
    LocationToAccountId,
    AccountId,
    NoChecking,
    CheckingAccount,
>;

/// `AssetId`/`Balance` converter for `ForeignAssets`
pub type ForeignAssetsConvertedConcreteId =
    assets_common::ForeignAssetsConvertedConcreteId<(), Balance, xcm::v3::MultiLocation>;

/// Means for transacting assets on this chain.
pub type AssetTransactors = (NativeAssetTransactor, ForeignAssetTransactor);

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// biases the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
    // Sovereign account converter; this attempts to derive an `AccountId` from the origin location
    // using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
    // foreign chains who want to have a local sovereign account on this chain which they control.
    SovereignSignedViaLocation<LocationToAccountId, RuntimeOrigin>,
    // Native converter for Relay-chain (Parent) location; will converts to a `Relay` origin when
    // recognized.
    RelayChainAsNative<RelayChainOrigin, RuntimeOrigin>,
    // Native converter for sibling Parachains; will convert to a `SiblingPara` origin when
    // recognized.
    SiblingParachainAsNative<cumulus_pallet_xcm::Origin, RuntimeOrigin>,
    // Native signed account converter; this just converts an `AccountId32` origin into a normal
    // `Origin::Signed` origin of the same 32-byte value.
    SignedAccountId32AsNative<RelayNetwork, RuntimeOrigin>,
    // Xcm origins can be represented natively under the Xcm pallet's Xcm origin.
    XcmPassthrough<RuntimeOrigin>,
);

parameter_types! {
    // One XCM operation is 1_000_000_000 weight - almost certainly a conservative estimate.
    // The default POV size used by Polkadot/Kusama was 64 kB but that has been updated here: https://github.com/paritytech/polkadot/pull/7081
    // We should properly benchmark instructions and get rid of fixed weights.
    pub UnitWeightCost: Weight = Weight::from_parts(1_000_000_000, 1024);
    pub const MaxInstructions: u32 = 100;
    pub const MaxAssetsIntoHolding: u32 = 64;
}

pub struct ParentOrParentsExecutivePlurality;
impl Contains<Location> for ParentOrParentsExecutivePlurality {
    fn contains(location: &Location) -> bool {
        matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
    }
}

pub type Barrier = (
    // Weight that is paid for may be consumed.
    TakeWeightCredit,
    // Expected responses are OK.
    AllowKnownQueryResponses<PolkadotXcm>,
    WithComputedOrigin<
        (
            // If the message is one that immediately attemps to pay for execution, then allow it.
            AllowTopLevelPaidExecutionFrom<Everything>,
            // Subscriptions for version tracking are OK.
            AllowSubscriptionsFrom<Everything>,
        ),
        UniversalLocation,
        ConstU32<8>,
    >,
);

/// A call filter for the XCM Transact instruction. This is a temporary measure until we
/// properly account for proof size weights.
///
/// Calls that are allowed through this filter must:
/// 1. Have a fixed weight;
/// 2. Cannot lead to another call being made;
/// 3. Have a defined proof size weight, e.g. no unbounded vecs in call parameters.
pub struct SafeCallFilter;
impl Contains<RuntimeCall> for SafeCallFilter {
    fn contains(call: &RuntimeCall) -> bool {
        #[cfg(feature = "runtime-benchmarks")]
        {
            if matches!(
                call,
                RuntimeCall::System(frame_system::Call::remark_with_event { .. })
            ) {
                return true;
            }
        }

        match call {
            RuntimeCall::System(
                frame_system::Call::kill_prefix { .. } | frame_system::Call::set_heap_pages { .. },
            )
            | RuntimeCall::Timestamp(..)
            | RuntimeCall::Balances(..)
            | RuntimeCall::Assets(..)
            | RuntimeCall::ForeignAssets(..)
            | RuntimeCall::Session(pallet_session::Call::purge_keys { .. })
            | RuntimeCall::Treasury(..)
            | RuntimeCall::Vesting(..)
            | RuntimeCall::Utility(pallet_utility::Call::as_derivative { .. })
            | RuntimeCall::Identity(
                pallet_identity::Call::add_registrar { .. }
                | pallet_identity::Call::set_identity { .. }
                | pallet_identity::Call::clear_identity { .. }
                | pallet_identity::Call::request_judgement { .. }
                | pallet_identity::Call::cancel_request { .. }
                | pallet_identity::Call::set_fee { .. }
                | pallet_identity::Call::set_account_id { .. }
                | pallet_identity::Call::set_fields { .. }
                | pallet_identity::Call::provide_judgement { .. }
                | pallet_identity::Call::kill_identity { .. }
                | pallet_identity::Call::add_sub { .. }
                | pallet_identity::Call::rename_sub { .. }
                | pallet_identity::Call::remove_sub { .. }
                | pallet_identity::Call::quit_sub { .. },
            )
            | RuntimeCall::Wrapper(..)
            | RuntimeCall::PolkadotXcm(..) => true,
            _ => false,
        }
    }
}

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    // How to withdraw and deposit an asset.
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = NativeAsset;
    type IsTeleporter = (); // Teleporting is disabled.
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type Trader = UsingComponents<WeightToFee, TokenLocation, AccountId, Balances, DealWithFees>;
    type ResponseHandler = PolkadotXcm;
    type AssetTrap = PolkadotXcm;
    type AssetClaims = PolkadotXcm;
    type SubscriptionService = PolkadotXcm;
    type PalletInstancesInfo = AllPalletsWithSystem;
    type MaxAssetsIntoHolding = MaxAssetsIntoHolding;
    type AssetLocker = ();
    type AssetExchanger = ();
    type FeeManager = ();
    type MessageExporter = ();
    type UniversalAliases = Nothing;
    type CallDispatcher = WithOriginFilter<SafeCallFilter>;
    type SafeCallFilter = SafeCallFilter;
    type Aliasers = Nothing;
    type TransactionalProcessor = ();
    type HrmpNewChannelOpenRequestHandler = ();
    type HrmpChannelAcceptedHandler = ();
    type HrmpChannelClosingHandler = ();
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = (
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, (), ()>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
);

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub ReachableDest: Option<Location> = Some(Parent.into());
}

impl pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type SendXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmRouter = XcmRouter;
    type ExecuteXcmOrigin = EnsureXcmOrigin<RuntimeOrigin, LocalOriginToLocation>;
    type XcmExecuteFilter = Nothing;
    // ^ Disable dispatchable execute on the XCM pallet.
    // Needs to be `Everything` for local testing.
    type XcmExecutor = XcmExecutor<XcmConfig>;
    type XcmTeleportFilter = Nothing;
    type XcmReserveTransferFilter = Everything;
    type Weigher = FixedWeightBounds<UnitWeightCost, RuntimeCall, MaxInstructions>;
    type UniversalLocation = UniversalLocation;
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    const VERSION_DISCOVERY_QUEUE_SIZE: u32 = 100;
    // ^ Override for AdvertisedXcmVersion default
    type AdvertisedXcmVersion = pallet_xcm::CurrentXcmVersion;
    type Currency = Balances;
    type CurrencyMatcher = ();
    type TrustedLockers = ();
    type SovereignAccountOf = LocationToAccountId;
    type MaxLockers = ConstU32<8>;
    type WeightInfo = crate::weights::pallet_xcm::WeightInfo<Runtime>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type AdminOrigin = EnsureRoot<AccountId>;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

// The parts below are copied from a later version of polkadot-sdk
// Copied from
// https://github.com/paritytech/polkadot-sdk/blob/7ef027551fd1290c42581a85052b643bffc9cbe4/polkadot/xcm/xcm-builder/src/location_conversion.rs
/// Converts locations from external global consensus systems (e.g., Ethereum, other parachains)
/// into `AccountId`.
///
/// Replaces `GlobalConsensusParachainConvertsFor` and `EthereumLocationsConverterFor` in a
/// backwards-compatible way, and extends them for also handling child locations (e.g.,
/// `AccountId(Alice)`).
pub struct ExternalConsensusLocationsConverterFor<UniversalLocation, AccountId>(
    PhantomData<(UniversalLocation, AccountId)>,
);

impl<UniversalLocation: Get<InteriorLocation>, AccountId: From<[u8; 32]> + Clone>
    ConvertLocation<AccountId>
    for ExternalConsensusLocationsConverterFor<UniversalLocation, AccountId>
{
    fn convert_location(location: &Location) -> Option<AccountId> {
        let universal_source = UniversalLocation::get();
        tracing::trace!(
            target: "xcm::location_conversion",
            "ExternalConsensusLocationsConverterFor universal_source: {:?}, location: {:?}",
            universal_source, location,
        );
        let (remote_network, remote_location) =
            ensure_is_remote(universal_source, location.clone()).ok()?;

        // replaces and extends `EthereumLocationsConverterFor` and
        // `GlobalConsensusParachainConvertsFor`
        let acc_id: AccountId = if let Ethereum { chain_id } = &remote_network {
            match remote_location.as_slice() {
                // equivalent to `EthereumLocationsConverterFor`
                [] => (b"ethereum-chain", chain_id)
                    .using_encoded(blake2_256)
                    .into(),
                // equivalent to `EthereumLocationsConverterFor`
                [AccountKey20 { network: _, key }] => (b"ethereum-chain", chain_id, *key)
                    .using_encoded(blake2_256)
                    .into(),
                // extends `EthereumLocationsConverterFor`
                tail => (b"ethereum-chain", chain_id, tail)
                    .using_encoded(blake2_256)
                    .into(),
            }
        } else {
            match remote_location.as_slice() {
                // equivalent to `GlobalConsensusParachainConvertsFor`
                [Parachain(para_id)] => (b"glblcnsnss/prchn_", remote_network, para_id)
                    .using_encoded(blake2_256)
                    .into(),
                // converts everything else based on hash of encoded location tail
                tail => (b"glblcnsnss", remote_network, tail)
                    .using_encoded(blake2_256)
                    .into(),
            }
        };
        Some(acc_id)
    }
}

// Copied from
// https://github.com/paritytech/polkadot-sdk/blob/a15d066faac70676101854cfa9b55f00f61e865a/polkadot/xcm/xcm-builder/src/universal_exports.rs#L34
/// Returns the network ID and consensus location within that network of the remote
/// location `dest` which is itself specified as a location relative to the local
/// chain, itself situated at `universal_local` within the consensus universe. If
/// `dest` is not a location in remote consensus, then an error is returned.
pub fn ensure_is_remote(
    universal_local: impl Into<InteriorLocation>,
    dest: impl Into<Location>,
) -> Result<(NetworkId, InteriorLocation), Location> {
    let dest = dest.into();
    let universal_local = universal_local.into();
    let local_net = match universal_local.global_consensus() {
        Ok(x) => x,
        Err(_) => return Err(dest),
    };
    let universal_destination: InteriorLocation = universal_local
        .into_location()
        .appended_with(dest.clone())
        .map_err(|x| x.1)?
        .try_into()?;
    let (remote_dest, remote_net) = match universal_destination.split_first() {
        (d, Some(GlobalConsensus(n))) if n != local_net => (d, n),
        _ => return Err(dest),
    };
    Ok((remote_net, remote_dest))
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
    pub const TrustedTeleporter: Option<(Location, Asset)> = Some((
        RelayLocation::get(),
        Asset { fun: Fungible(EXISTENTIAL_DEPOSIT), id: AssetId(RelayLocation::get()) },
    ));
    pub const CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = None;
    pub TrustedReserve: Option<(Location, Asset)> = Some((
        RelayLocation::get(),
        Asset { fun: Fungible(UNITS), id: AssetId(RelayLocation::get()) },
    ));
}

#[cfg(feature = "runtime-benchmarks")]
impl pallet_xcm_benchmarks::Config for Runtime {
    type XcmConfig = XcmConfig;
    type AccountIdConverter = LocationToAccountId;
    type DeliveryHelper = ();
    fn valid_destination() -> Result<Location, frame_benchmarking::BenchmarkError> {
        Ok(RelayLocation::get())
    }
    fn worst_case_holding(_depositable_count: u32) -> xcm::latest::Assets {
        let asset = Asset {
            id: AssetId(TokenLocation::get()),
            fun: Fungible(1_000_000 * UNITS),
        };
        vec![asset].into()
    }
}

#[cfg(feature = "runtime-benchmarks")]
impl pallet_xcm_benchmarks::fungible::Config for Runtime {
    type TransactAsset = Balances;
    type CheckedAccount = CheckedAccount;
    type TrustedTeleporter = TrustedTeleporter;
    type TrustedReserve = TrustedReserve;

    fn get_asset() -> Asset {
        Asset {
            id: AssetId(TokenLocation::get()),
            fun: Fungible(10 * EXISTENTIAL_DEPOSIT),
        }
    }
}

#[cfg(feature = "runtime-benchmarks")]
impl pallet_xcm_benchmarks::generic::Config for Runtime {
    type TransactAsset = Balances;
    type RuntimeCall = RuntimeCall;

    fn worst_case_response() -> (u64, Response) {
        (0u64, Response::Version(Default::default()))
    }

    fn worst_case_asset_exchange() -> Result<(Assets, Assets), frame_benchmarking::BenchmarkError> {
        Err(frame_benchmarking::BenchmarkError::Skip)
    }

    fn universal_alias() -> Result<(Location, Junction), frame_benchmarking::BenchmarkError> {
        Err(frame_benchmarking::BenchmarkError::Skip)
    }

    fn transact_origin_and_runtime_call(
    ) -> Result<(Location, RuntimeCall), frame_benchmarking::BenchmarkError> {
        Ok((
            RelayLocation::get(),
            frame_system::Call::remark_with_event { remark: vec![] }.into(),
        ))
    }

    fn subscribe_origin() -> Result<Location, frame_benchmarking::BenchmarkError> {
        Ok(RelayLocation::get())
    }

    fn claimable_asset() -> Result<(Location, Location, Assets), frame_benchmarking::BenchmarkError>
    {
        let origin = RelayLocation::get();
        let assets: Assets = (AssetId(RelayLocation::get()), 1_000 * UNITS).into();
        let ticket = Location {
            parents: 0,
            interior: Here,
        };
        Ok((origin, ticket, assets))
    }

    fn fee_asset() -> Result<Asset, frame_benchmarking::BenchmarkError> {
        Ok(Asset {
            id: AssetId(TokenLocation::get()),
            fun: Fungible(EXISTENTIAL_DEPOSIT),
        })
    }

    fn unlockable_asset() -> Result<(Location, Location, Asset), frame_benchmarking::BenchmarkError>
    {
        Err(frame_benchmarking::BenchmarkError::Skip)
    }

    fn export_message_origin_and_destination(
    ) -> Result<(Location, NetworkId, Junctions), frame_benchmarking::BenchmarkError> {
        Err(frame_benchmarking::BenchmarkError::Skip)
    }

    fn alias_origin() -> Result<(Location, Location), frame_benchmarking::BenchmarkError> {
        Err(frame_benchmarking::BenchmarkError::Skip)
    }
}
