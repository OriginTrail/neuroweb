use pallet_evm::{ExitRevert, IsPrecompileResult, Precompile, PrecompileFailure, PrecompileHandle, PrecompileResult, PrecompileSet};
use sp_core::H160;
use sp_std::marker::PhantomData;

use pallet_evm_precompile_assets_erc20::{AddressToAssetId, Erc20AssetsPrecompileSet};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};

/// The asset precompile address prefix. Addresses that match against this prefix will be routed
/// to Erc20AssetsPrecompileSet
pub const ASSET_PRECOMPILE_ADDRESS_PREFIX: &[u8] = &[255u8; 4];

pub struct FrontierPrecompiles<R>(PhantomData<R>);

impl<R> FrontierPrecompiles<R>
where
	R: pallet_evm::Config,
{
	pub fn new() -> Self {
		Self(Default::default())
	}
	/// Return all addresses that contain precompiles. This can be used to populate dummy code
    /// under the precompile.
    pub fn used_addresses() -> impl Iterator<Item = H160> {
        sp_std::vec![1, 2, 3, 4, 5, 1024, 1025]
            .into_iter()
            .map(hash)
    }
}
impl<R> PrecompileSet for FrontierPrecompiles<R>
where
	Erc20AssetsPrecompileSet<R>: PrecompileSet,
	R: pallet_evm::Config
		+ pallet_assets::Config
        + AddressToAssetId<<R as pallet_assets::Config>::AssetId>,
{
	fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<PrecompileResult> {
		let address = handle.code_address();
		if let IsPrecompileResult::Answer { is_precompile, .. } =
            self.is_precompile(address, u64::MAX)
        {
            if is_precompile && address > hash(9) && handle.context().address != address {
                return Some(Err(PrecompileFailure::Revert {
                    exit_status: ExitRevert::Reverted,
                    output: b"cannot be called with DELEGATECALL or CALLCODE".to_vec(),
                }));
            }
        }
		match address {
			// Ethereum precompiles :
			a if a == hash(1) => Some(ECRecover::execute(handle)),
			a if a == hash(2) => Some(Sha256::execute(handle)),
			a if a == hash(3) => Some(Ripemd160::execute(handle)),
			a if a == hash(4) => Some(Identity::execute(handle)),
			a if a == hash(5) => Some(Modexp::execute(handle)),
			// Non-Frontier specific nor Ethereum precompiles :
			a if a == hash(1024) => Some(Sha3FIPS256::execute(handle)),
			a if a == hash(1025) => Some(ECRecoverPublicKey::execute(handle)),
			// If the address matches asset prefix, the we route through the asset precompile set
            a if &a.to_fixed_bytes()[0..4] == ASSET_PRECOMPILE_ADDRESS_PREFIX => {
                Erc20AssetsPrecompileSet::<R>::new().execute(handle)
            }
			_ => None,
		}
	}

	fn is_precompile(&self, address: H160, gas: u64) -> IsPrecompileResult {
        let assets_precompile =
            match Erc20AssetsPrecompileSet::<R>::new().is_precompile(address, gas) {
                IsPrecompileResult::Answer { is_precompile, .. } => is_precompile,
                _ => false,
            };

        IsPrecompileResult::Answer {
            is_precompile: assets_precompile || Self::used_addresses().any(|x| x == address),
            extra_cost: 0,
        }
    }
}

fn hash(a: u64) -> H160 {
	H160::from_low_u64_be(a)
}