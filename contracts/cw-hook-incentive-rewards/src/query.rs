use cosmwasm_std::{Addr, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;
use cw_utils::NativeBalance;
use kujira::bow::staking::IncentivesResponse;
use rewards_interfaces::{PendingRewardsResponse, StakeInfoResponse};
use rewards_logic::incentive;

use crate::{contract::STATE_MACHINE, Config, ContractError};

pub fn pending_rewards(
    deps: Deps,
    env: Env,
    config: &Config,
    staker: Addr,
) -> Result<PendingRewardsResponse, ContractError> {
    let lri = incentive::get_lri(deps.storage, config.incentive_crank_limit, &env.block.time)?;
    let (_, lri_user) = STATE_MACHINE
        .calculate_users_rewards(deps.storage, &vec![staker.to_string()], &lri)?
        .pop()
        .unwrap();
    let accrued = STATE_MACHINE.get_accrued(deps.storage, &staker.to_string())?;
    let mut accrued = NativeBalance(accrued) + NativeBalance(lri_user);
    accrued.normalize();
    Ok(PendingRewardsResponse {
        rewards: accrued.into_vec(),
    })
}

pub fn stake_info(deps: Deps, staker: Addr) -> Result<StakeInfoResponse, ContractError> {
    let amount = STATE_MACHINE
        .user_weights
        .may_load(deps.storage, &staker.to_string())?
        .unwrap_or_default();
    Ok(StakeInfoResponse { staker, amount })
}

pub fn incentives(
    deps: Deps,
    start_after: Option<Uint128>,
    limit: Option<u32>,
) -> Result<IncentivesResponse, ContractError> {
    let is = incentive::incentives()
        .range(
            deps.storage,
            start_after.map(Bound::exclusive),
            None,
            Order::Ascending,
        )
        .take(limit.unwrap_or(30) as usize)
        .map(|r| r.map(|(_, v)| v.into()))
        .collect::<StdResult<Vec<_>>>()?;
    Ok(IncentivesResponse { incentives: is })
}
