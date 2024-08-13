use cosmwasm_std::{Addr, Deps, Env, Order, StdResult, Uint128};
use cw_rewards_logic::{incentive, inflation, PendingRewardsResponse, StakeInfoResponse};
use cw_storage_plus::Bound;
use cw_utils::NativeBalance;
use kujira::bow::staking::IncentivesResponse;

use crate::{contract::STATE_MACHINE, msg::InflationResponse, Config, ContractError};

pub fn pending_rewards(
    deps: Deps,
    env: Env,
    config: &Config,
    staker: Addr,
) -> Result<PendingRewardsResponse, ContractError> {
    let mut accrued = STATE_MACHINE.get_accrued(deps.storage, &staker.to_string())?;
    if let Some(incentive_cfg) = &config.incentive_module {
        let lri = incentive::get_lri(deps.storage, incentive_cfg.crank_limit, &env.block.time)?;
        let (_, lri_user) = STATE_MACHINE
            .calculate_users_rewards(deps.storage, &vec![staker.to_string()], &lri)?
            .pop()
            .unwrap();

        accrued = (NativeBalance(accrued) + NativeBalance(lri_user)).into_vec();
    }
    if let Some(underlying_cfg) = &config.underlying_rewards_module {
        let underlying_rewards: PendingRewardsResponse = deps.querier.query_wasm_smart(
            &underlying_cfg.underlying_rewards_contract,
            &crate::msg::QueryMsg::PendingRewards {
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
        accrued = (NativeBalance(accrued) + NativeBalance(pending_user)).into_vec();
    }
    if let Some(inflation_cfg) = &config.inflation_module {
        if let Some((inflation, _)) = inflation::pending_inflation(
            deps.storage,
            &STATE_MACHINE,
            &inflation_cfg.rate_per_year,
            &env.block.time,
        )? {
            let (_, inflation_user) = STATE_MACHINE
                .calculate_users_rewards(deps.storage, &vec![staker.to_string()], &vec![inflation])?
                .pop()
                .unwrap();
            accrued = (NativeBalance(accrued) + NativeBalance(inflation_user)).into_vec();
        }
    }

    Ok(PendingRewardsResponse { rewards: accrued })
}

pub fn stake_info(deps: Deps, staker: Addr) -> Result<StakeInfoResponse, ContractError> {
    let amount = STATE_MACHINE
        .user_weights
        .may_load(deps.storage, &staker.to_string())?
        .unwrap_or_default();
    Ok(StakeInfoResponse { staker, amount })
}

pub fn weights(
    deps: Deps,
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

pub fn inflation(
    deps: Deps,
    env: Env,
    config: &Config,
) -> Result<InflationResponse, ContractError> {
    if let Some(inflation_cfg) = &config.inflation_module {
        let inflation = inflation::pending_inflation(
            deps.storage,
            &STATE_MACHINE,
            &inflation_cfg.rate_per_year,
            &env.block.time,
        )?;
        Ok(InflationResponse {
            rate_per_year: inflation_cfg.rate_per_year,
            funds: inflation.map(|(_, remaining)| remaining),
        })
    } else {
        Err(ContractError::InflationNotEnabled {})
    }
}
