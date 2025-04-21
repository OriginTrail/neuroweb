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
    keccak256, revert, succeed, Address, Bytes, EvmData, EvmDataReader, EvmDataWriter, EvmResult,
    FunctionModifier, LogExt, LogsBuilder, PrecompileHandleExt, RuntimeHelper,
};
use sp_core::{Get, H160, U256};
use sp_runtime::traits::{Bounded, Dispatchable, Zero};
use sp_std::{
    convert::{TryFrom, TryInto},
    marker::PhantomData,
    vec::Vec,
};

pub type BalanceOf<Runtime> = <Runtime as pallet_parachain_staking::Config>::Balance;

#[precompile_utils::generate_function_selector]
#[derive(Debug, PartialEq)]
pub enum Action {
    MinDelegation = "min_delegation()",
    Points = "points(uint256)",
    CandidateCount = "candidate_count()",
    Round = "round()",
    CandidateDelegationCount = "candidate_delegation_count(address)",
    DelegatorDelegationCount = "delegator_delegation_count(address)",
    JoinCandidates = "join_candidates(uint256,uint256)",
    Delegate = "delegate(address,uint256,uint256,uint256)",
    ScheduleLeaveDelegators = "schedule_leave_delegators()",
    ExecuteLeaveDelegators = "execute_leave_delegators(address,uint256)",
    CancelLeaveDelegators = "cancel_leave_delegators()",
    DelegatorBondMore = "delegator_bond_more(address,uint256)",
    ExecuteDelegationRequest = "execute_delegation_request(address,address)",
    CancelDelegationRequest = "cancel_delegation_request(address)",
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
    Runtime::RuntimeCall: From<pallet_parachain_staking::Call<Runtime>>,
    <<Runtime as frame_system::Config>::RuntimeCall as Dispatchable>::RuntimeOrigin: OriginTrait,
{
    fn execute(&self, handle: &mut impl PrecompileHandle) -> Option<EvmResult<PrecompileOutput>> {
        let selector = match handle.read_selector() {
            Ok(selector) => selector,
            Err(e) => return Some(Err(e)),
        };

        if let Err(err) = handle.check_function_modifier(match selector {
            Action::MinDelegation
            | Action::Points
            | Action::CandidateCount
            | Action::Round
            | Action::CandidateDelegationCount
            | Action::DelegatorDelegationCount => FunctionModifier::View,
            Action::Delegate
            | Action::JoinCandidates
            | Action::ScheduleLeaveDelegators
            | Action::ExecuteLeaveDelegators
            | Action::CancelLeaveDelegators
            | Action::DelegatorBondMore
            | Action::ExecuteDelegationRequest
            | Action::CancelDelegationRequest => FunctionModifier::NonPayable,
        }) {
            return Some(Err(err));
        }

        let result = match selector {
            Action::MinDelegation => Self::min_delegation(handle),
            Action::Points => Self::points(handle),
            Action::CandidateCount => Self::candidate_count(handle),
            Action::Round => Self::round(handle),
            Action::CandidateDelegationCount => Self::candidate_delegation_count(handle),
            Action::DelegatorDelegationCount => Self::delegator_delegation_count(handle),
            Action::JoinCandidates => Self::join_candidates(handle),
            Action::Delegate => Self::delegate(handle),
            Action::ScheduleLeaveDelegators => Self::schedule_leave_delegators(handle),
            Action::ExecuteLeaveDelegators => Self::execute_leave_delegators(handle),
            Action::CancelLeaveDelegators => Self::cancel_leave_delegators(handle),
            Action::DelegatorBondMore => Self::delegator_bond_more(handle),
            Action::ExecuteDelegationRequest => Self::execute_delegation_request(handle),
            Action::CancelDelegationRequest => Self::cancel_delegation_request(handle),
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
    Runtime::RuntimeCall: From<pallet_parachain_staking::Call<Runtime>>,
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

    fn points(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;
        // Read input.
        input.expect_arguments(1)?;
        let round = input.read::<u32>()?;

        // Fetch info.
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        let points: u32 = pallet_parachain_staking::Pallet::<Runtime>::points(round);

        // Build output.
        Ok(succeed(EvmDataWriter::new().write(points).build()))
    }

    fn candidate_count(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // Fetch info.
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        let candidate_count: u32 = <pallet_parachain_staking::Pallet<Runtime>>::candidate_pool()
            .0
            .len() as u32;

        // Build output.
        Ok(succeed(EvmDataWriter::new().write(candidate_count).build()))
    }

    fn round(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        // Fetch info.
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        let round: u32 = <pallet_parachain_staking::Pallet<Runtime>>::round().current;

        // Build output.
        Ok(succeed(EvmDataWriter::new().write(round).build()))
    }

    fn candidate_delegation_count(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;
        // Read input.
        input.expect_arguments(1)?;
        let address = input.read::<Address>()?.0;
        let address = Runtime::AddressMapping::into_account_id(address);

        // Fetch info.
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        let result = if let Some(state) =
            <pallet_parachain_staking::Pallet<Runtime>>::candidate_info(&address)
        {
            let candidate_delegation_count: u32 = state.delegation_count;

            log::trace!(
                target: "staking-precompile",
                "Result from pallet is {:?}",
                candidate_delegation_count
            );
            candidate_delegation_count
        } else {
            log::trace!(
                target: "staking-precompile",
                "Candidate {:?} not found, so delegation count is 0",
                address
            );
            0u32
        };

        // Build output.
        Ok(succeed(EvmDataWriter::new().write(result).build()))
    }

    fn delegator_delegation_count(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;
        // Read input.
        input.expect_arguments(1)?;
        let address = input.read::<Address>()?.0;
        let address = Runtime::AddressMapping::into_account_id(address);

        // Fetch info.
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        let result = if let Some(state) =
            <pallet_parachain_staking::Pallet<Runtime>>::delegator_state(&address)
        {
            let delegator_delegation_count: u32 = state.delegations.0.len() as u32;

            log::trace!(
                target: "staking-precompile",
                "Result from pallet is {:?}",
                delegator_delegation_count
            );

            delegator_delegation_count
        } else {
            log::trace!(
                target: "staking-precompile",
                "Delegator {:?} not found, so delegation count is 0",
                address
            );
            0u32
        };

        // Build output.
        Ok(succeed(EvmDataWriter::new().write(result).build()))
    }

    fn join_candidates(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;

        // Read input.
        input.expect_arguments(2)?;
        let bond: BalanceOf<Runtime> = input.read()?;
        let candidate_count = input.read()?;

        {
            // Build call with origin.
            let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);

            RuntimeHelper::<Runtime>::try_dispatch(
                handle,
                Some(origin).into(),
                pallet_parachain_staking::Call::<Runtime>::join_candidates {
                    bond,
                    candidate_count,
                },
            )?;
        }

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn delegate(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;
        // Read input.
        input.expect_arguments(4)?;
        let candidate = Runtime::AddressMapping::into_account_id(input.read::<Address>()?.0);
        let amount: BalanceOf<Runtime> = input.read()?;
        let candidate_delegation_count = input.read()?;
        let delegation_count = input.read()?;

        {
            // Build call with origin.
            let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
            let call = pallet_parachain_staking::Call::<Runtime>::delegate {
                candidate,
                amount,
                candidate_delegation_count,
                delegation_count,
            };

            RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        }

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn schedule_leave_delegators(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        {
            // Build call with origin.
            let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
            let call = pallet_parachain_staking::Call::<Runtime>::schedule_leave_delegators {};

            RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        }

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn execute_leave_delegators(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;
        // Read input.
        input.expect_arguments(2)?;
        let delegator = input.read::<Address>()?.0;
        let delegator = Runtime::AddressMapping::into_account_id(delegator);
        let delegation_count = input.read()?;

        {
            // Build call with origin.
            let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
            let call = pallet_parachain_staking::Call::<Runtime>::execute_leave_delegators {
                delegator,
                delegation_count,
            };

            RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        }

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn cancel_leave_delegators(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        {
            // Build call with origin.
            let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
            let call = pallet_parachain_staking::Call::<Runtime>::cancel_leave_delegators {};

            RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        }

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn delegator_bond_more(handle: &mut impl PrecompileHandle) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;
        // Read input.
        input.expect_arguments(2)?;
        let candidate = input.read::<Address>()?.0;
        let candidate = Runtime::AddressMapping::into_account_id(candidate);
        let more: BalanceOf<Runtime> = input.read()?;

        {
            // Build call with origin.
            let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
            let call =
                pallet_parachain_staking::Call::<Runtime>::delegator_bond_more { candidate, more };

            RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        }

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn execute_delegation_request(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;
        // Read input.
        input.expect_arguments(2)?;
        let delegator = input.read::<Address>()?.0;
        let delegator = Runtime::AddressMapping::into_account_id(delegator);
        let candidate = input.read::<Address>()?.0;
        let candidate = Runtime::AddressMapping::into_account_id(candidate);

        {
            // Build call with origin.
            let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
            let call = pallet_parachain_staking::Call::<Runtime>::execute_delegation_request {
                delegator,
                candidate,
            };

            RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        }

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }

    fn cancel_delegation_request(
        handle: &mut impl PrecompileHandle,
    ) -> EvmResult<PrecompileOutput> {
        let mut input = EvmDataReader::new_skip_selector(handle.input())?;
        // Read input.
        input.expect_arguments(1)?;
        let candidate = input.read::<Address>()?.0;
        let candidate = Runtime::AddressMapping::into_account_id(candidate);

        {
            // Build call with origin.
            let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
            let call =
                pallet_parachain_staking::Call::<Runtime>::cancel_delegation_request { candidate };

            RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        }

        Ok(succeed(EvmDataWriter::new().write(true).build()))
    }
}
