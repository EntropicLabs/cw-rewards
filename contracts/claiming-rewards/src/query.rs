use cosmwasm_std::{Addr, Deps, Env};
use cw_utils::NativeBalance;
use kujira::KujiraQuery;
use rewards_interfaces::{PendingRewardsResponse, StakeInfoResponse};

use crate::{contract::STATE_MACHINE, Config, ContractError};

pub fn pending_rewards(
    deps: Deps<KujiraQuery>,
    env: Env,
    config: &Config,
    staker: Addr,
) -> Result<PendingRewardsResponse, ContractError> {
    let underlying_rewards: PendingRewardsResponse = deps.querier.query_wasm_smart(
        &config.underlying_rewards,
        &rewards_interfaces::permissioned_incentive::QueryMsg::PendingRewards {
            staker: env.contract.address,
        },
    )?;
    let (_, pending_user) = STATE_MACHINE
        .calculate_users_rewards(
            deps.storage,
            &vec![staker.to_string()],
            &underlying_rewards.rewards,
        )?
        .pop()
        .unwrap();
    let accrued = STATE_MACHINE.get_accrued(deps.storage, &staker.to_string())?;
    let mut accrued = NativeBalance(accrued) + NativeBalance(pending_user);
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
