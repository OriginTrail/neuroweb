# DPoS Pallet for Parachain Staking

The parachain-staking pallet provides minimal staking functionality, enabling direct delegation for collator selection based on total backed stake. Unlike frame/pallet-staking, this pallet does not use Phragmen for delegation but allows delegators to choose exactly who to back. This repository contains a modified version of Moonbeam's and Polimec's 'parachain_staking' Substrate Pallet.

## General Rules

### Rounds

* A new round starts every <Round<T>>::get().length blocks.
* At the start of each round:
    * Rewards for block authoring from T::RewardPaymentDelay rounds ago are calculated.
    * A new set of collators is selected from the candidates.

### Joining and Leaving

* Collators
    * To join as a collator candidate, an account must bond at least MinCandidateStk.
    * To leave, the account must call schedule_leave_candidates, and the exit is executable after T::LeaveCandidatesDelay rounds.

* Delegators
    * To delegate, an account must bond at least MinDelegatorStk.
    * Delegators can revoke a delegation or leave entirely by calling the relevant extrinsics.

### Reward Distribution

* Rewards are distributed with a delay of T::RewardPaymentDelay rounds.
* Collators and their top T::MaxTopDelegationsPerCandidate delegators receive rewards proportionally based on their stake.

### Selection

* Each round, a fixed number (T::TotalSelected) of collators are chosen based on their total backing stake (self-bond + delegations).

## Extrinsics and Parameters

### General Extrinsics

* set_staking_expectations(expectations: Range<BalanceOf<T>>)
    * Purpose: Sets the expected total stake range for collators and delegators.
    * Parameters:
        * expectations: Defines minimum, ideal, and maximum total stake values.

* set_inflation(schedule: Range<Perbill>)
    * Purpose: Configures the annual inflation rate to determine per-round issuance.
    * Parameters:
        * schedule: Defines minimum, ideal, and maximum inflation rates.

* set_blocks_per_round(new: u32)
    * Purpose: Sets the number of blocks per round.
    * Parameters:
        * new: The new block length for a round.

### Collator-Specific Extrinsics

* join_candidates(bond: BalanceOf<T>, candidate_count: u32)
    * Purpose: Allows an account to become a collator candidate.
    * Parameters:
        * bond: The amount of funds to bond as a self-stake.
        * candidate_count: The current number of candidates in the pool (used as a weight hint).

* schedule_leave_candidates(candidate_count: u32)
    * Purpose: Requests to leave the set of collators.
    * Parameters:
        * candidate_count: The current number of candidates in the pool (used as a weight hint).

* execute_leave_candidates(candidate: T::AccountId, candidate_delegation_count: u32)
    * Purpose: Executes the leave request for a collator candidate.
    * Parameters:
        * candidate: The account of the collator.
        * candidate_delegation_count: The number of delegators backing the collator (used as a weight hint).

* candidate_bond_more(more: BalanceOf<T>)
    * Purpose: Increases the self-bond of a collator candidate.
    * Parameters:
        * more: The additional amount to bond.

* schedule_candidate_bond_less(less: BalanceOf<T>)
    * Purpose: Schedules a decrease in the self-bond of a collator candidate.
    * Parameters:
        * less: The amount to decrease.

* execute_candidate_bond_less(candidate: T::AccountId)
    * Purpose: Executes a scheduled decrease in the self-bond of a collator.
    * Parameters:
        * candidate: The account of the collator.

### Delegator-Specific Extrinsics

* delegate(candidate: T::AccountId, amount: BalanceOf<T>, candidate_delegation_count: u32, delegation_count: u32)
    * Purpose: Allows an account to delegate stake to a collator candidate.
    * Parameters:
        * candidate: The account of the collator to delegate to.
        * amount: The amount of funds to delegate.
        * candidate_delegation_count: The number of delegators already backing the collator (used as a weight hint).
        * delegation_count: The number of delegations by the delegator (used as a weight hint).

* schedule_revoke_delegation(collator: T::AccountId)
    * Purpose: Schedules the revocation of a delegation to a collator.
    * Parameters:
        * collator: The account of the collator.

* execute_delegation_request(delegator: T::AccountId, candidate: T::AccountId)
    * Purpose: Executes a scheduled delegation revocation or decrease.
    * Parameters:
        * delegator: The account of the delegator.
        * candidate: The account of the collator.

* delegator_bond_more(candidate: T::AccountId, more: BalanceOf<T>)
    * Purpose: Increases the bond of a delegator to a specific collator.
    * Parameters:
        * candidate: The account of the collator.
        * more: The additional amount to bond.

* schedule_delegator_bond_less(candidate: T::AccountId, less: BalanceOf<T>)
    * Purpose: Schedules a decrease in the bond of a delegator to a specific collator.
    * Parameters:
        * candidate: The account of the collator.
        * less: The amount to decrease.

* schedule_leave_delegators()
    * Purpose: Schedules the exit of a delegator from all delegations.

* execute_leave_delegators(delegator: T::AccountId, delegation_count: u32)
    * Purpose: Executes the exit of a delegator from all delegations.
    * Parameters:
        * delegator: The account of the delegator.
        * delegation_count: The number of delegations by the delegator (used as a weight hint).


### Storage Items

* CandidateInfo: Stores metadata for collator candidates.
* DelegatorState: Stores the state of delegators.
* AtStake: Snapshots collator delegation stake at the start of each round.
* DelayedPayouts: Stores delayed payout information for rewards.
* Points: Tracks points awarded to collators for block production.

### Events

* NewRound: Emitted when a new round begins.
* Rewarded: Emitted when a reward is paid to an account.
* CollatorChosen: Emitted when a collator is selected for the next round.
* Delegation: Emitted when a new delegation is made.

## Modifications
The modifications to the original pallet include the following:
1. Removed Nimbus Dependencies: The original dependencies on Nimbus have been removed. This simplifies the usage of the pallet and makes it independent of Nimbus.
2. Implemented Traits from **pallet_authorship** and **pallet_session**: To replace some functionality previously provided by Nimbus, several traits from _pallet_authorship_ and _pallet_session_ have been implemented:
    - **EventHandler** from *pallet_authorship*: This trait is used to note the block author and award them points for producing a block. The points are then used for staking purposes.q
    - **SessionManager** from *pallet_session*: This trait is used to manage the start and end of sessions, as well as assemble new collators for new sessions.
    - **ShouldEndSession** from *pallet_session*: This trait is used to decide when a session should end.
    - **EstimateNextSessionRotation** from *pallet_session*: This trait is used to estimate the average session length and the current session progress, as well as estimate the next session rotation.