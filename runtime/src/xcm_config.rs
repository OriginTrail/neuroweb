use super::{
    AccountId, AllPalletsWithSystem, Balance, Balances, DealWithFees, ForeignAssets, ParachainInfo,
    ParachainSystem, PolkadotXcm, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, WeightToFee,
    XcmpQueue,
};
use crate::Vec;
use codec::Encode;
use core::marker::PhantomData;

use frame_support::{
    parameter_types,
    traits::{
        fungibles::Mutate, ConstU32, Contains, ContainsPair, Everything, Get, Nothing,
        PalletInfoAccess,
    },
    weights::constants::WEIGHT_REF_TIME_PER_SECOND,
};
use frame_system::EnsureRoot;
use pallet_xcm::XcmPassthrough;
use polkadot_parachain::primitives::Sibling;
use scale_info::prelude::vec;
use sp_core::blake2_256;
use xcm::v4::prelude::*;
use xcm::v4::InteriorLocation;
use xcm_builder::{
    AccountId32Aliases, AllowKnownQueryResponses, AllowSubscriptionsFrom,
    AllowTopLevelPaidExecutionFrom, Case, EnsureXcmOrigin, FungibleAdapter, FungiblesAdapter,
    IsConcrete, NativeAsset, NoChecking, ParentIsPreset, RelayChainAsNative,
    SiblingParachainAsNative, SiblingParachainConvertsVia, SignedAccountId32AsNative,
    SignedToAccountId32, SovereignSignedViaLocation, TakeWeightCredit, TrailingSetTopicAsId,
    UsingComponents, WeightInfoBounds, WithComputedOrigin, WithUniqueTopic,
};
use xcm_executor::{
    traits::{
        ConvertLocation, DropAssets, Error as MatchError, MatchesFungibles, WeightTrader,
        WithOriginFilter,
    },
    AssetsInHolding, XcmExecutor,
};

parameter_types! {
    // NEURO (native)
    pub TokenLocation: Location = Location {
        parents:0,
        interior: [
            PalletInstance(<Balances as PalletInfoAccess>::index() as u8)
        ].into()
    };

    pub const RelayLocation: Location = Location::parent();
    pub const RelayNetwork: NetworkId = Polkadot;
    pub RelayChainOrigin: RuntimeOrigin = cumulus_pallet_xcm::Origin::Relay.into();

    /// Asset Hub
    pub AssetHubLocation: Location = (Parent, Parachain(1000)).into();
    pub RelayChainNativeAssetFromAssetHub: (AssetFilter, Location) =
        (Wild(AllOf {
            fun: WildFungible,
            id: xcm::prelude::AssetId(RelayLocation::get()),
           }),
        AssetHubLocation::get()
    );

    pub UniversalLocation: InteriorLocation = [GlobalConsensus(RelayNetwork::get()), Parachain(ParachainInfo::parachain_id().into())].into();
    pub CheckingAccount: AccountId = PolkadotXcm::check_account();

    // XCM fees in DOT
    pub DotPerSecond: u128 = 1_000_000_000; // 0.1 DOT/sec, in Planks
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
    IsForeignConcreteAssetFrom<AssetHubLocation>,
    LocationToAccountId,
    AccountId,
    NoChecking,
    CheckingAccount,
>;

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

/// A trader which accepts DOT (via RelayLocation) at a fixed rate.
pub struct FixedRateOfForeignAsset<AssetLocation, Balance, Rate>(
    PhantomData<(AssetLocation, Balance, Rate)>,
);

impl<
        AssetLocation: Get<Location>,
        Balance: From<u128> + Into<u128> + Copy + Ord + Default + core::ops::Sub,
        Rate: Get<u128>,
    > WeightTrader for FixedRateOfForeignAsset<AssetLocation, Balance, Rate>
{
    fn new() -> Self {
        FixedRateOfForeignAsset(PhantomData)
    }

    fn buy_weight(
        &mut self,
        weight: Weight,
        payment: AssetsInHolding,
        _ctx: &XcmContext,
    ) -> Result<AssetsInHolding, XcmError> {
        let asset_id = AssetId(AssetLocation::get());
        let rate_per_second = Rate::get();

        let fee: u128 = (weight.ref_time() as u128)
            .saturating_mul(rate_per_second)
            .checked_div(WEIGHT_REF_TIME_PER_SECOND as u128)
            .ok_or(XcmError::Overflow)?;
        let fee_balance: Balance = fee.into();

        let assets: Assets = payment.clone().into();

        let mut found_amount: Option<Balance> = None;
        for asset in assets.clone().into_inner().into_iter() {
            if asset.id == asset_id {
                if let Fungible(amount) = asset.fun {
                    found_amount = Some(amount.into());
                    break;
                }
            }
        }

        let amount = found_amount.ok_or(XcmError::TooExpensive)?;
        if amount < fee_balance {
            return Err(XcmError::TooExpensive);
        }

        // Subtract fee and rebuild
        let mut new_assets: Vec<Asset> = Vec::new();
        let leftover: u128 = amount.into() - fee_balance.into();

        if leftover > 0 {
            new_assets.push(Asset {
                id: asset_id.clone(),
                fun: Fungible(leftover),
            });
        }

        // Instead of OnUnbalanced, just drop fees via your DealWithForeignFees
        DealWithForeignFees::drop_assets(
            &Location::here(),
            vec![Asset {
                id: asset_id.clone(),
                fun: Fungible(fee_balance.into()),
            }]
            .into(),
            _ctx,
        );

        Ok(Assets::from(new_assets).into())
    }

    fn refund_weight(&mut self, _weight: Weight, _ctx: &XcmContext) -> Option<Asset> {
        None
    }
}

// Deposit fees into the Treasury
// TODO: Change to TakeRevenue after SDK upgrade
pub struct DealWithForeignFees;
impl DropAssets for DealWithForeignFees {
    fn drop_assets(_origin: &Location, assets: AssetsInHolding, _ctx: &XcmContext) -> Weight {
        let assets: Assets = assets.into();

        for asset in assets.into_inner().into_iter() {
            // Only handle DOT (Relay Location)
            if asset.id == AssetId(RelayLocation::get()) {
                if let Fungible(amount) = asset.fun {
                    if amount > 0 {
                        // Credit Treasury account in pallet-assets (asset_id = Location)
                        match ForeignAssets::mint_into(
                            RelayLocation::get(), // directly use Location as ID
                            &crate::Treasury::account_id(),
                            amount,
                        ) {
                            Ok(_) => {
                                log::info!(
                                    target: "xcm::fees",
                                    "Credited {} DOT into Treasury account ({:?})",
                                    amount,
                                     &crate::Treasury::account_id(),
                                );
                            }
                            Err(e) => {
                                log::warn!(
                                    target: "xcm::fees",
                                    "Failed to credit DOT fees ({}) into Treasury ({:?}): {:?}",
                                    amount,
                                    &crate::Treasury::account_id(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        Weight::zero()
    }
}

parameter_types! {
    pub const MaxInstructions: u32 = 100;
    pub const MaxAssetsIntoHolding: u32 = 64;
}

pub struct ParentOrParentsExecutivePlurality;
impl Contains<Location> for ParentOrParentsExecutivePlurality {
    fn contains(location: &Location) -> bool {
        matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
    }
}

pub type Barrier = TrailingSetTopicAsId<(
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
)>;

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

/// Matches foreign assets from a given origin.
/// Foreign assets are assets bridged from other consensus systems. i.e parents > 1.
pub struct IsForeignConcreteAssetFrom<Origin>(PhantomData<Origin>);

impl<Origin> MatchesFungibles<Location, u128> for IsForeignConcreteAssetFrom<Origin>
where
    Origin: Get<Location>,
{
    fn matches_fungibles(asset: &Asset) -> Result<(Location, u128), MatchError> {
        let expected_origin = Origin::get();

        let AssetId(asset_location) = &asset.id;

        // Ensure DOT comes from Asset Hub
        if *asset_location == RelayLocation::get() && expected_origin == AssetHubLocation::get() {
            if let Fungible(amount) = asset.fun {
                return Ok((asset_location.clone(), amount));
            }
        }

        // Handle assets from Ethereum
        if expected_origin == AssetHubLocation::get() && asset_location.parents == 2 {
            if let Some(first_junction) = asset_location.interior.first() {
                if matches!(first_junction, GlobalConsensus(Ethereum { .. })) {
                    if let Fungible(amount) = asset.fun {
                        return Ok((asset_location.clone(), amount));
                    }
                }
            }
        }

        Err(MatchError::AssetNotHandled)
    }
}
impl<Origin> ContainsPair<Asset, Location> for IsForeignConcreteAssetFrom<Origin>
where
    Origin: Get<Location>,
{
    fn contains(asset: &Asset, origin: &Location) -> bool {
        let loc = Origin::get();
        &loc == origin
            && matches!(
                asset,
                Asset {
                    id: AssetId(Location { parents: 2, .. }),
                    fun: Fungibility::Fungible(_)
                }
            )
    }
}

type Reserves = (
    // Relaychain (DOT) from Asset Hub
    Case<RelayChainNativeAssetFromAssetHub>,
    // Assets bridged from different consensus systems held in reserve on Asset Hub.
    IsForeignConcreteAssetFrom<AssetHubLocation>,
    // Assets which the reserve is the same as the origin.
    NativeAsset,
);

pub struct XcmConfig;
impl xcm_executor::Config for XcmConfig {
    type RuntimeCall = RuntimeCall;
    type XcmSender = XcmRouter;
    // How to withdraw and deposit an asset.
    type AssetTransactor = AssetTransactors;
    type OriginConverter = XcmOriginToTransactDispatchOrigin;
    type IsReserve = Reserves;
    type IsTeleporter = (); // Teleporting is disabled.
    type UniversalLocation = UniversalLocation;
    type Barrier = Barrier;
    type Weigher = WeightInfoBounds<
        crate::weights::xcm::NeurowebXcmWeight<RuntimeCall>,
        RuntimeCall,
        MaxInstructions,
    >;
    type Trader = (
        UsingComponents<WeightToFee, TokenLocation, AccountId, Balances, DealWithFees>,
        FixedRateOfForeignAsset<RelayLocation, Balance, DotPerSecond>,
    );
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
    type XcmRecorder = PolkadotXcm;
}

/// No local origins on this chain are allowed to dispatch XCM sends/executions.
pub type LocalOriginToLocation = SignedToAccountId32<RuntimeOrigin, AccountId, RelayNetwork>;

/// The means for routing XCM messages which are not for local execution into the right message
/// queues.
pub type XcmRouter = WithUniqueTopic<(
    // Two routers - use UMP to communicate with the relay chain:
    cumulus_primitives_utility::ParentAsUmp<ParachainSystem, (), ()>,
    // ..and XCMP to communicate with the sibling chains.
    XcmpQueue,
)>;

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
    type Weigher = WeightInfoBounds<
        crate::weights::xcm::NeurowebXcmWeight<RuntimeCall>,
        RuntimeCall,
        MaxInstructions,
    >;
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
    type WeightInfo = crate::weights::pallet_xcm::NeurowebWeight<Runtime>;
    type MaxRemoteLockConsumers = ConstU32<0>;
    type RemoteLockConsumerIdentifier = ();
    type AdminOrigin = EnsureRoot<AccountId>;
}

impl cumulus_pallet_xcm::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type XcmExecutor = XcmExecutor<XcmConfig>;
}

// The parts below are copied from a later version of polkadot-sdk
//
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
mod benchmarking {
    use super::*;
    use crate::xcm_config::benchmarking;
    use crate::{assets::EXISTENTIAL_DEPOSIT, Box, UNITS};
    use cumulus_primitives_core::ParaId;
    use frame_support::traits::Currency;

    parameter_types! {
        pub const RandomParaId: ParaId = ParaId::new(123);
        pub const ExistentialDeposit: u128 = 1_000_000_000_000;
    }

    impl pallet_xcm::benchmarking::Config for Runtime {
        type DeliveryHelper = ();

        fn reachable_dest() -> Option<Location> {
            Some(Parent.into())
        }

        fn teleportable_asset_and_dest() -> Option<(Asset, Location)> {
            None
        }

        fn reserve_transferable_asset_and_dest() -> Option<(Asset, Location)> {
            // open a channel to a random sibling for benchmarking
            ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(
                benchmarking::RandomParaId::get(),
            );

            let who = frame_benchmarking::whitelisted_caller();
            let balance = 10 * benchmarking::ExistentialDeposit::get();
            let _ = <Balances as Currency<_>>::make_free_balance_be(&who, balance);

            Some((
                Asset {
                    id: AssetId(TokenLocation::get()),
                    fun: Fungible(benchmarking::ExistentialDeposit::get() / 10),
                },
                // destination: parent → sibling parachain
                (Parent, Parachain(benchmarking::RandomParaId::get().into())).into(),
            ))
        }

        fn set_up_complex_asset_transfer() -> Option<(Assets, u32, Location, Box<dyn FnOnce()>)> {
            let para_id = benchmarking::RandomParaId::get();

            ParachainSystem::open_outbound_hrmp_channel_for_benchmarks_or_tests(para_id);

            let destination: Location = (Parent, Parachain(para_id.into())).into();

            let fee_asset: Asset = (TokenLocation::get(), ExistentialDeposit::get()).into();
            let transfer_asset: Asset = (TokenLocation::get(), ExistentialDeposit::get()).into();

            let who = frame_benchmarking::whitelisted_caller();
            let balance = 10 * benchmarking::ExistentialDeposit::get();
            let _ = Balances::make_free_balance_be(&who, balance);
            let assets: Assets = vec![fee_asset.clone(), transfer_asset].into();
            let fee_index: u32 = 0;

            let ed = benchmarking::ExistentialDeposit::get();
            let verify: Box<dyn FnOnce()> = Box::new(move || {
                assert!(Balances::free_balance(&who) <= balance - ed);
            });

            Some((assets, fee_index, destination, verify))
        }

        fn get_asset() -> Asset {
            Asset {
                id: AssetId(TokenLocation::get()),
                fun: Fungible(benchmarking::ExistentialDeposit::get()),
            }
        }
    }

    parameter_types! {
        pub const TrustedTeleporter: Option<(Location, Asset)> = Some((
            RelayLocation::get(),
            Asset { fun: Fungible(EXISTENTIAL_DEPOSIT), id: AssetId(RelayLocation::get()) },
        ));
        pub CheckedAccount: Option<(AccountId, xcm_builder::MintLocation)> = Some((
            CheckingAccount::get(),
            xcm_builder::MintLocation::Local,
        ));
        pub TrustedReserve: Option<(Location, Asset)> = Some((
            RelayLocation::get(),
            Asset { fun: Fungible(UNITS), id: AssetId(RelayLocation::get()) },
        ));
    }

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

    impl pallet_xcm_benchmarks::fungible::Config for Runtime {
        type TransactAsset = Balances;
        type CheckedAccount = CheckedAccount;
        type TrustedTeleporter = TrustedTeleporter;
        type TrustedReserve = TrustedReserve;

        fn get_asset() -> Asset {
            use frame_support::traits::Currency;

            // Fund the CheckedAccount for benchmarks
            if let Some((checked_account, _)) = CheckedAccount::get() {
                let balance = 1000 * benchmarking::ExistentialDeposit::get();
                let _ = <Balances as Currency<_>>::make_free_balance_be(&checked_account, balance);
            }

            Asset {
                id: AssetId(TokenLocation::get()),
                fun: Fungible(benchmarking::ExistentialDeposit::get()),
            }
        }
    }

    impl pallet_xcm_benchmarks::generic::Config for Runtime {
        type TransactAsset = Balances;
        type RuntimeCall = RuntimeCall;

        fn worst_case_response() -> (u64, Response) {
            (0u64, Response::Version(Default::default()))
        }

        fn worst_case_asset_exchange(
        ) -> Result<(Assets, Assets), frame_benchmarking::BenchmarkError> {
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

        fn claimable_asset(
        ) -> Result<(Location, Location, Assets), frame_benchmarking::BenchmarkError> {
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

        fn unlockable_asset(
        ) -> Result<(Location, Location, Asset), frame_benchmarking::BenchmarkError> {
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
}
