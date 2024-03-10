use cosmwasm_std::{Addr, Deps, Order, StdResult, Uint128};
use cw_storage_plus::Bound;
use kujira::{bow::staking::IncentivesResponse, KujiraQuery};
use rewards_interfaces::{PendingRewardsResponse, StakeInfoResponse};

use crate::{contract::STATE_MACHINE, incentive, ContractError};

pub fn pending_rewards(
    deps: Deps<KujiraQuery>,
    staker: Addr,
) -> Result<PendingRewardsResponse, ContractError> {
    let accrued = STATE_MACHINE.get_accrued(deps.storage, &staker.to_string())?;
    Ok(PendingRewardsResponse { rewards: accrued })
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
