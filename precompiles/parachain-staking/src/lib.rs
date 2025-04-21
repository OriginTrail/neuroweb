// Copyright 2019-2022 PureStake Inc.
// Copyright 2022      Stake Technologies
// Copyright 2022      TraceLabs
// This file is part of AssetsERC20 package, originally developed by Purestake Inc.
// AssetsERC20 package used in NeuroWeb Parachain in terms of GPLv3.
//
// AssetsERC20 is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// AssetsERC20 is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with AssetsERC20.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(test, feature(assert_matches))]

use fp_evm::{IsPrecompileResult, PrecompileHandle, PrecompileOutput};
use frame_support::traits::fungibles::approvals::Inspect as ApprovalInspect;
use frame_support::traits::fungibles::metadata::Inspect as MetadataInspect;
use frame_support::traits::fungibles::Inspect;
use frame_support::traits::OriginTrait;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    sp_runtime::traits::StaticLookup,
};
use pallet_evm::{AddressMapping, PrecompileSet};
use precompile_utils::{
    keccak256, revert, succeed, Address, Bytes, EvmData, EvmDataWriter, EvmResult,
    FunctionModifier, LogExt, LogsBuilder, PrecompileHandleExt, RuntimeHelper,
};
use sp_runtime::traits::{Bounded, Dispatchable, Zero};

use sp_core::{Get, H160, U256};
use sp_std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
};

pub type BalanceOf<Runtime> = <Runtime as pallet_parachain_staking::Config>::Balance;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    MinDelegation = "min_delegation()",
    Round = "round()",
}
pub struct ParachainStakingPrecompileSet<Runtime>(PhantomData<Runtime>);

impl<Runtime> ParachainStakingPrecompileSet<Runtime> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<Runtime> PrecompileSet for ParachainStakingPrecompileSet<Runtime>
where
    Runtime: pallet_parachain_staking::Config + pallet_evm::Config + frame_system::Config,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + EvmData,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<EvmResult<PrecompileOutput>> {
        let selector = match handle.read_selector() {
            Ok(selector) => selector,
            Err(e) => return Some(Err(e)),
        };

        if let Err(err) = handle.check_function_modifier(match selector {
            Action::MinDelegation | Action::Round => FunctionModifier::NonPayable,
            _ => FunctionModifier::View,
        }) {
            return Some(Err(err));
        }

        let result = match selector {
            Action::MinDelegation => Self::min_delegation(handle),
            Action::Round => Self::round(handle),
        };

        return Some(result);
    }

    fn is_precompile(&self, _address: H160, _gas: u64) -> IsPrecompileResult {
        IsPrecompileResult::Answer {
            is_precompile: true,
            extra_cost: 0,
        }
    }
}

impl<Runtime> ParachainStakingPrecompileSet<Runtime>
where
    Runtime: pallet_parachain_staking::Config + pallet_evm::Config + frame_system::Config,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + EvmData,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
    fn min_delegation(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // Fetch info.
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        let min_nomination: u128 =
            <<Runtime as pallet_parachain_staking::Config>::MinDelegation as Get<
                BalanceOf<Runtime>,
            >>::get()
            .try_into()
            .map_err(|_| revert("Amount is too large for provided balance type"))?;

        // Build output.
        Ok(succeed(EvmDataWriter::new().write(min_nomination).build()))
    }

    fn round(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // Fetch info.
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        let round: u32 = <pallet_parachain_staking::Pallet<Runtime>>::round().current;

        // Build output.
        Ok(succeed(EvmDataWriter::new().write(round).build()))
    }
}
