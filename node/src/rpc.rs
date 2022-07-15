//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use origintrail_parachain_runtime::{opaque::Block, AccountId, Hash, Balance, Index as Nonce};

use sc_client_api::{
	backend::{AuxStore, Backend, StorageProvider},
	client::BlockchainEvents,
};
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sc_network::NetworkService;

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies
pub struct FullDeps<C, P> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// Network service
	pub network: Arc<NetworkService<Block, Hash>>,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, BE>(
	deps: FullDeps<C, P>,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	BE: Backend<Block> + 'static,
	C: ProvideRuntimeApi<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Send
		+ Sync
		+ 'static,
	C: StorageProvider<Block, BE>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Nonce>,
	C::Api: BlockBuilder<Block>,
	C::Api: fp_rpc::EthereumRuntimeRPCApi<Block>,
	P: TransactionPool + Sync + Send + 'static,
{
	use pallet_transaction_payment_rpc::{TransactionPaymentApiServer, TransactionPaymentRpc};
	use substrate_frame_rpc_system::{SystemApiServer, SystemRpc};
	use fc_rpc::{NetApi, NetApiServer};

	let mut module = RpcExtension::new(());
	let FullDeps { client, pool, deny_unsafe, network } = deps;

	module.merge(SystemRpc::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPaymentRpc::new(client.clone()).into_rpc())?;

	io.merge(NetApi::new(
		client.clone(),
		network.clone(),
		// Whether to format the `peer_count` response as Hex (default) or not.
		true,
	).into_rpc());

	Ok(module)
}
