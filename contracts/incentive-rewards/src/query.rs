use cosmwasm_std::{Addr, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;
use cw_utils::NativeBalance;
use kujira::{bow::staking::IncentivesResponse, KujiraQuery};
use rewards_interfaces::{PendingRewardsResponse, StakeInfoResponse};
use rewards_logic::incentive;

use crate::{contract::STATE_MACHINE, Config, ContractError};

pub fn pending_rewards(
    deps: Deps<KujiraQuery>,
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

pub fn stake_info(
    deps: Deps<KujiraQuery>,
    staker: Addr,
) -> Result<StakeInfoResponse, ContractError> {
    let amount = STATE_MACHINE
        .user_weights
        .may_load(deps.storage, &staker.to_string())?
        .unwrap_or_default();
    Ok(StakeInfoResponse { staker, amount })
}

pub fn weights(
    deps: Deps<KujiraQuery>,
    start_after: Option<Addr>,
    limit: Option<u32>,
) -> Result<Vec<StakeInfoResponse>, ContractError> {
    let start_after = start_after.map(|a| a.to_string());
    let weights = STATE_MACHINE
        .user_weights
        .range(
            deps.storage,
            start_after.as_ref().map(Bound::exclusive),
            None,
            Order::Ascending,
        )
        .take(limit.unwrap_or(30) as usize)
        .map(|r| {
            r.map(|(k, v)| StakeInfoResponse {
                staker: Addr::unchecked(k),
                amount: v,
            })
        })
        .collect::<StdResult<Vec<_>>>()?;
    Ok(weights)
}

pub fn incentives(
    deps: Deps<KujiraQuery>,
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
