//! A collection of node-specific RPC methods.
//! Substrate provides the `sc-rpc` crate, which defines the core RPC layer
//! used by Substrate nodes. This file extends those RPC definitions with
//! capabilities that are specific to this project's runtime configuration.

#![warn(missing_docs)]

use std::sync::Arc;

use origintrail_parachain_runtime::{opaque::Block, AccountId, Balance, Hash, Index as Nonce};

use sc_client_api::{
	backend::{AuxStore, Backend, StateBackend, StorageProvider},
};
pub use sc_rpc::{DenyUnsafe, SubscriptionTaskExecutor};
use fc_rpc::{
	EthBlockDataCacheTask, OverrideHandle, EthFilter, EthFilterApiServer,
};
use sp_runtime::traits::BlakeTwo256;
use fc_rpc_core::types::{FeeHistoryCache, FilterPool};
use sc_transaction_pool::{ChainApi, Pool};
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sc_network::NetworkService;

/// A type representing all RPC extensions.
pub type RpcExtension = jsonrpsee::RpcModule<()>;

/// Full client dependencies
pub struct FullDeps<C, P, A: ChainApi> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// Graph pool instance.
	pub graph: Arc<Pool<A>>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// The Node authority flag
    pub is_authority: bool,
	/// Network service
	pub network: Arc<NetworkService<Block, Hash>>,
	/// Backend.
	pub backend: Arc<fc_db::Backend<Block>>,
	/// EthFilterApi pool.
    pub filter_pool: FilterPool,
	/// Maximum fee history cache size.                                                                                    
    pub fee_history_cache_limit: u64,
    /// Fee history cache.
    pub fee_history_cache: FeeHistoryCache,
	/// Ethereum data access overrides.
	pub overrides: Arc<OverrideHandle<Block>>,
    /// Cache for Ethereum block data.
	pub block_data_cache: Arc<EthBlockDataCacheTask<Block>>,
}

/// Instantiate all RPC extensions.
pub fn create_full<C, P, BE, A>(
	deps: FullDeps<C, P, A>,
	_subscription_task_executor: SubscriptionTaskExecutor,
) -> Result<RpcExtension, Box<dyn std::error::Error + Send + Sync>>
where
	BE: Backend<Block> + 'static,
	BE::State: StateBackend<BlakeTwo256>,
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
	C::Api: fp_rpc::ConvertTransactionRuntimeApi<Block>,
	P: TransactionPool<Block = Block> + Sync + Send + 'static,
	P: TransactionPool + Sync + Send + 'static,
	A: ChainApi<Block = Block> + 'static,
{
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};
	use fc_rpc::{Eth, EthApiServer, Net, NetApiServer};

	let mut module = RpcExtension::new(());
	let FullDeps { client, pool, graph, deny_unsafe, network, backend, is_authority, filter_pool,
		fee_history_cache, fee_history_cache_limit, overrides, block_data_cache
	} = deps;

	module.merge(System::new(client.clone(), pool.clone(), deny_unsafe).into_rpc())?;
	module.merge(TransactionPayment::new(client.clone()).into_rpc())?;

	let signers = Vec::new();

	module.merge(
        Eth::<_, _, _, fp_rpc::NoTransactionConverter, _, _, _>::new(
            client.clone(),
			pool.clone(),
            graph,
            None,
            network.clone(),
            signers,
            overrides.clone(),
			backend.clone(),
            is_authority,
            block_data_cache.clone(),
            fee_history_cache,
            fee_history_cache_limit,
        ).into_rpc()
    )?;

	let max_past_logs: u32 = 10_000;
    let max_stored_filters: usize = 500;
    module.merge(
        EthFilter::new(
            client.clone(),
            backend,
            filter_pool,
            max_stored_filters,
            max_past_logs,
            block_data_cache,
        )
        .into_rpc(),
    )?;

	module.merge(
		Net::new(
			client.clone(),
			network.clone(),
			// Whether to format the `peer_count` response as Hex (default) or not.
			true,
		)
		.into_rpc(),
	)?;

	Ok(module)
}
