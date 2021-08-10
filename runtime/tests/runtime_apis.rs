mod common;
use common::*;

use nimbus_primitives::NimbusId;
use pallet_evm::{Account as EVMAccount, AddressMapping, FeeCalculator, GenesisAccount};
use sp_core::{Public, H160, H256, U256};
use origintrail_parachain_runtime::{GLMR };
use fp_rpc::runtime_decl_for_EthereumRuntimeRPCApi::EthereumRuntimeRPCApi;
use fp_rpc::ConvertTransaction;
use frame_support::assert_noop;
use moonbeam_rpc_primitives_debug::runtime_decl_for_DebugRuntimeApi::DebugRuntimeApi;
use moonbeam_rpc_primitives_txpool::runtime_decl_for_TxPoolRuntimeApi::TxPoolRuntimeApi;
use std::collections::BTreeMap;
use std::str::FromStr;

fn ethereum_transaction() -> pallet_ethereum::Transaction {
	// {from: 0x6be02d1d3665660d22ff9624b7be0551ee1ac91b, .., gasPrice: "0x01"}
	let bytes = hex::decode(
		"f86880843b9aca0083b71b0094111111111111111111111111111111111111111182020080820a26a0\
		8c69faf613b9f72dbb029bb5d5acf42742d214c79743507e75fdc8adecdee928a001be4f58ff278ac61\
		125a81a582a717d9c5d6554326c01b878297c6522b12282",
	)
	.expect("Transaction bytes.");
	let transaction = rlp::decode::<pallet_ethereum::Transaction>(&bytes[..]);
	assert!(transaction.is_ok());
	transaction.unwrap()
}

fn uxt() -> UncheckedExtrinsic {
	let converter = TransactionConverter;
	converter.convert_transaction(ethereum_transaction())
}

#[test]
fn ethereum_runtime_rpc_api_chain_id() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Runtime::chain_id(), CHAIN_ID);
	});
}

#[test]
fn ethereum_runtime_rpc_api_account_basic() {
	ExtBuilder::default()
		.with_balances(vec![(AccountId::from(ALICE), 2_000 * GLMR)])
		.build()
		.execute_with(|| {
			assert_eq!(
				Runtime::account_basic(H160::from(ALICE)),
				EVMAccount {
					balance: U256::from(2_000 * GLMR),
					nonce: U256::zero()
				}
			);
		});
}

#[test]
fn ethereum_runtime_rpc_api_gas_price() {
	ExtBuilder::default().build().execute_with(|| {
		assert_eq!(Runtime::gas_price(), FixedGasPrice::min_gas_price());
	});
}

#[test]
fn ethereum_runtime_rpc_api_account_code_at() {
	let address = H160::from(EVM_CONTRACT);
	let code: Vec<u8> = vec![1, 2, 3, 4, 5];
	ExtBuilder::default()
		.with_evm_accounts({
			let mut map = BTreeMap::new();
			map.insert(
				address,
				GenesisAccount {
					balance: U256::zero(),
					code: code.clone(),
					nonce: Default::default(),
					storage: Default::default(),
				},
			);
			map
		})
		.build()
		.execute_with(|| {
			assert_eq!(Runtime::account_code_at(address), code);
		});
}