use cumulus_primitives_core::ParaId;
use origintrail_parachain_runtime::{AccountId, Signature, EVMConfig, EthereumConfig, GLMR, InflationInfo, Range, AuthorFilterConfig,
									AuthorMappingConfig, Balance, BalancesConfig, EthereumChainIdConfig, CouncilConfig, 
									GenesisConfig, ParachainInfoConfig, SudoConfig, SystemConfig, WASM_BINARY, ParachainStakingConfig};

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use sc_service::ChainType;
use serde::{Deserialize, Serialize};
use sp_core::{H160, U256, Pair, Public, sr25519};
use sp_runtime::{
	traits::{BlakeTwo256, Hash, IdentifyAccount, Verify},
	Perbill, Percent
};


use pallet_evm::GenesisAccount;
use std::str::FromStr;
use serde_json as json;
use nimbus_primitives::NimbusId;

use std::convert::TryInto;


/// Specialized `ChainSpec` for the normal parachain runtime.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

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

pub fn development_config(para_id: ParaId) -> ChainSpec {
	ChainSpec::from_genesis(
		// Name
		"Development",
		// ID
		"dev",
		ChainType::Local,
		move || {
			testnet_genesis(
				AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap(),
				// Validator
				vec![(
					AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap(),
					get_from_seed::<NimbusId>("Alice"),
					// SBP M3 review: use UNITS or DOLLARS instead of GLMR ?
					1_000 * GLMR,
				)],
				// Nominations
				vec![],
				moonbeam_inflation_config(),
				vec![
					AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap(),
					AccountId::from_str("81D288F95a78bc074ea7e831DEb6B046fEb3ef61").unwrap(),
					AccountId::from_str("9AAc88b9FD5C4ff74DCe3775dAe365856Aa14064").unwrap(),
					AccountId::from_str("76DF50d897eE66473D91df327cCB2690bccb5551").unwrap(),
					AccountId::from_str("413E9c0e2F92A8b939833f1bc3c48135aff767b5").unwrap(),
				],
				Default::default(), // para_id
				2160,               //ChainId
			)
		},
		vec![],
		None,
		None,
		// SPP M3 review: you could specify the tokenSymbol here as well
		Some(serde_json::from_str("{\"tokenDecimals\": 18}").expect("Provided valid json map")),
		Extensions {
			relay_chain: "dev-service".into(),
			para_id: para_id.into(),
		},
	)
}

pub fn local_testnet_config(para_id: ParaId) -> ChainSpec {
	ChainSpec::from_genesis(
		// Name
		"Local Testnet",
		// ID
		"local_testnet",
		ChainType::Local,
		move || {
			testnet_genesis(
				AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap(),
				// Validator
				vec![(
					AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap(),
					get_from_seed::<NimbusId>("Alice"),
					// SBP M3 review: use UNITS or DOLLARS instead of GLMR ?
					1_000 * GLMR,
				)],
				// Nominations
				vec![],
				moonbeam_inflation_config(),
				vec![AccountId::from_str("6Be02d1d3665660d22FF9624b7BE0551ee1Ac91b").unwrap()],
				para_id,
				2160, //ChainId
			)
		},
		vec![],
		None,
		None,
		// SPP M3 review: you could specify the tokenSymbol here as well
		Some(serde_json::from_str("{\"tokenDecimals\": 18}").expect("Provided valid json map")),
		Extensions {
			relay_chain: "local_testnet".into(),
			para_id: para_id.into(),
		},
	)
}

// SBP M3 review: rename to starfleet_inflation_config ?
pub fn moonbeam_inflation_config() -> InflationInfo<Balance> {
	InflationInfo {
		expect: Range {
			// SBP M3 review: use UNITS or DOLLARS instead of GLMR ?
			min: 100_000 * GLMR,
			ideal: 200_000 * GLMR,
			max: 500_000 * GLMR,
		},
		annual: Range {
			min: Perbill::from_percent(4),
			ideal: Perbill::from_percent(5),
			max: Perbill::from_percent(5),
		},
		// 8766 rounds (hours) in a year
		round: Range {
			min: Perbill::from_parts(Perbill::from_percent(4).deconstruct() / 8766),
			ideal: Perbill::from_parts(Perbill::from_percent(5).deconstruct() / 8766),
			max: Perbill::from_parts(Perbill::from_percent(5).deconstruct() / 8766),
		},
	}
}

pub fn testnet_genesis(
	root_key: AccountId,
	stakers: Vec<(AccountId, NimbusId, Balance)>,
	nominations: Vec<(AccountId, AccountId, Balance)>,
	inflation_config: InflationInfo<Balance>,
	endowed_accounts: Vec<AccountId>,
	para_id: ParaId,
	chain_id: u64,
) -> GenesisConfig {
	let revert_bytecode = vec![0x60, 0x00, 0x60, 0x00, 0xFD];

	let precompile_addresses = vec![1, 2, 3, 4, 5, 6, 7, 8, 1024, 1025, 2048]
		.into_iter()
		.map(H160::from_low_u64_be);

	GenesisConfig {
		system: SystemConfig {
			code: WASM_BINARY
				.expect("WASM binary was not build, please build it!")
				.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, 1 << 80))
				.collect(),
		},
		sudo: SudoConfig { key: root_key },
		parachain_info: ParachainInfoConfig { parachain_id: para_id },
		council: CouncilConfig::default(),
		council_membership: Default::default(),
		treasury: Default::default(),
		ethereum_chain_id: EthereumChainIdConfig { chain_id },
		evm: EVMConfig {
			accounts: precompile_addresses
				.map(|a| {
					(
						a,
						GenesisAccount {
							nonce: Default::default(),
							balance: Default::default(),
							storage: Default::default(),
							code: revert_bytecode.clone(),
						},
					)
				})
				.collect(),
		},
		ethereum: EthereumConfig {},
		parachain_staking: ParachainStakingConfig {
			candidates: stakers
				.iter()
				.cloned()
				.map(|(account, _, bond)| (account, bond))
				.collect(),
			nominations,
			inflation_config,
		},
		author_filter: AuthorFilterConfig { eligible_ratio: Percent::from_percent(50), },
		author_mapping: AuthorMappingConfig {
			mappings: stakers
				.iter()
				.cloned()
				.map(|(account_id, author_id, _)| (author_id, account_id))
				.collect(),
		},
	}
}
