use cumulus_primitives_core::ParaId;
use neuroweb_runtime::{pallet_parachain_staking::{
		inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR},
		InflationInfo, Range,
	}, AccountId, AuraId, Balance, EVMConfig, MinCandidateStk, Signature, OTP};
use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{sr25519, Pair, Public, H160, U256};
use sp_runtime::{Perbill, Percent, traits::{IdentifyAccount, Verify}};
use std::{collections::BTreeMap, str::FromStr};
use itertools::Itertools;

/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec =
	sc_service::GenericChainSpec<neuroweb_runtime::RuntimeGenesisConfig, Extensions>;

/// The default XCM version to set in genesis config.
pub const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
pub const COLLATOR_COMMISSION: Perbill = Perbill::from_percent(10);
pub const PARACHAIN_BOND_RESERVE_PERCENT: Percent = Percent::from_percent(0);
pub const BLOCKS_PER_ROUND: u32 = 3600; // 6 hours of blocks
pub const NUM_SELECTED_CANDIDATES: u32 = 1; // For start

pub fn neuroweb_inflation_config() -> InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
		perbill_annual_to_perbill_round(
			annual,
			// rounds per year
			BLOCKS_PER_YEAR / BLOCKS_PER_ROUND,
		)
	}
	let annual =
		Range { min: Perbill::from_percent(2), ideal: Perbill::from_percent(3), max: Perbill::from_percent(3) };

	InflationInfo {
		// staking expectations
		expect: Range { min: 100_000 * OTP, ideal: 200_000 * OTP, max: 500_000 * OTP },
		// annual inflation
		annual,
		round: to_round_inflation(annual),
	}
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// The extensions for the [`ChainSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

impl Extensions {
	/// Try to get the extension from the given `ChainSpec`.
	pub fn try_get(chain_spec: &dyn sc_service::ChainSpec) -> Option<&Self> {
		sc_chain_spec::get_extension(chain_spec.extensions())
	}
}

type AccountPublic = <Signature as Verify>::Signer;

/// Generate collator keys from seed.
///
/// This function's return type must always match the session keys of the chain in tuple format.
pub fn get_collator_keys_from_seed(seed: &str) -> AuraId {
	get_from_seed::<AuraId>(seed)
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn template_session_keys(keys: AuraId) -> neuroweb_runtime::SessionKeys {
	neuroweb_runtime::SessionKeys { aura: keys }
}

pub fn development_config() -> ChainSpec {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "NEURO".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("ss58Format".into(), 101.into());

	ChainSpec::builder(
		neuroweb_runtime::WASM_BINARY.expect("WASM binary was not build, please build it!"),
		Extensions {
			relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
			para_id: 2043,
		},
	)
	.with_name("Development")
	.with_id("dev")
	.with_chain_type(ChainType::Development)
	.with_properties(properties)
	.with_genesis_config(testnet_genesis(
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						get_collator_keys_from_seed("Alice"),
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_collator_keys_from_seed("Bob"),
					),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				2043.into(),
	))
	.build()
}

pub fn local_testnet_config() -> ChainSpec {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("tokenSymbol".into(), "NEURO".into());
	properties.insert("tokenDecimals".into(), 12.into());
	properties.insert("ss58Format".into(), 101.into());

	ChainSpec::builder(
		neuroweb_runtime::WASM_BINARY.expect("WASM binary was not build, please build it!"),
		Extensions {
			relay_chain: "rococo-local".into(), // You MUST set this to the correct network!
			para_id: 2043,
		},
	)
	.with_name("NeuroWeb Testnet")
	.with_id("neuroweb_testnet")
	.with_chain_type(ChainType::Local)
	.with_properties(properties)
	.with_genesis_config(testnet_genesis(
				vec![
					(
						get_account_id_from_seed::<sr25519::Public>("Alice"),
						get_collator_keys_from_seed("Alice"),
					),
					(
						get_account_id_from_seed::<sr25519::Public>("Bob"),
						get_collator_keys_from_seed("Bob"),
					),
				],
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				vec![
					get_account_id_from_seed::<sr25519::Public>("Alice"),
					get_account_id_from_seed::<sr25519::Public>("Bob"),
					get_account_id_from_seed::<sr25519::Public>("Charlie"),
					get_account_id_from_seed::<sr25519::Public>("Dave"),
					get_account_id_from_seed::<sr25519::Public>("Eve"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie"),
					get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
					get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
					get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
					get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
					get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
					get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
				],
				2043.into(),
	))
	.build()
}

fn testnet_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	_root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	let config = neuroweb_runtime::RuntimeGenesisConfig {
		system: Default::default(),
		balances: neuroweb_runtime::BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		parachain_info: neuroweb_runtime::ParachainInfoConfig { 
			parachain_id: id,
			..Default::default()
		},
		session: neuroweb_runtime::SessionConfig {
			keys: invulnerables.clone()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                 // account id
						acc,                         // validator id
						template_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		polkadot_xcm: neuroweb_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		vesting: Default::default(),
		treasury: Default::default(),
		evm: EVMConfig {
			accounts: {
				let mut map = BTreeMap::new();
				map.insert(
					// H160 address of Alice dev account
					// Derived from SS58 (42 prefix) address
					// SS58: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
					// hex: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
					// Using the full hex key, truncating to the first 20 bytes (the first 40 hex chars)
					H160::from_str("d43593c715fdd31c61141abd04a99fd6822c8558")
						.expect("internal H160 is valid; qed"),
					fp_evm::GenesisAccount {
						balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
							.expect("internal U256 is valid; qed"),
						code: Default::default(),
						nonce: Default::default(),
						storage: Default::default(),
					},
				);
				map.insert(
					// H160 address of CI test runner account
					H160::from_str("6be02d1d3665660d22ff9624b7be0551ee1ac91b")
						.expect("internal H160 is valid; qed"),
					fp_evm::GenesisAccount {
						balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
							.expect("internal U256 is valid; qed"),
						code: Default::default(),
						nonce: Default::default(),
						storage: Default::default(),
					},
				);
				map
			},
			..Default::default()
		},
		ethereum: Default::default(),
		base_fee: Default::default(),
		council: Default::default(),
		democracy: Default::default(),
		transaction_payment: Default::default(),
		parachain_staking: neuroweb_runtime::ParachainStakingConfig {
			candidates: invulnerables.clone().into_iter().map(|account| (account.0, MinCandidateStk::get())).collect_vec(),
			delegations: vec![],
			blocks_per_round: BLOCKS_PER_ROUND,
			num_selected_candidates: NUM_SELECTED_CANDIDATES,
			parachain_bond_reserve_percent: PARACHAIN_BOND_RESERVE_PERCENT,
			collator_commission: COLLATOR_COMMISSION,
			inflation_config: neuroweb_inflation_config()
		}
	};

	serde_json::to_value(&config).expect("Could not build genesis config.")
}
