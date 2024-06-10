use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Timestamp,
};
use cw2::set_contract_version;
use cw_utils::NativeBalance;

use rewards_interfaces::{
    incentive::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    RewardsMsg,
};
use rewards_logic::{
    incentive::{self, Incentive},
    RewardsSM,
};

use crate::{execute, query, Config, ContractError};

const CONTRACT_NAME: &str = "entropic/incentivized-rewards";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STATE_MACHINE: RewardsSM = RewardsSM::new();

#[cw_serde]
pub struct MigrateMsg {}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config::from(msg);
    config.save(deps.storage, deps.api)?;

    STATE_MACHINE.initialize(deps.storage)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut config = Config::load(deps.storage)?;
    match msg {
        ExecuteMsg::Rewards(msg) => {
            if !STATE_MACHINE
                .total_staked
                .may_load(deps.storage)?
                .unwrap_or_default()
                .is_zero()
            {
                incentive::distribute_lri(
                    deps.storage,
                    config.incentive_crank_limit,
                    STATE_MACHINE,
                    &env.block.time,
                )?;
            }
            match msg {
                RewardsMsg::Stake(msg) => execute::stake(deps, info, config, msg),
                RewardsMsg::Unstake(msg) => execute::unstake(deps, info, config, msg),
                RewardsMsg::ClaimRewards(msg) => execute::claim(deps, info, msg),
                RewardsMsg::DistributeRewards(msg) => execute::distribute(deps, info, config, msg),
            }
        }
        ExecuteMsg::AddIncentive { denom, schedule } => {
            let bal = NativeBalance(info.funds);
            let bal = (bal - config.incentive_fee)
                .map_err(|_| ContractError::InvalidIncentive {})?
                .0;
            if bal.len() != 1
                || bal[0].amount < config.incentive_min
                || bal[0].denom != denom.as_ref()
            {
                return Err(ContractError::InvalidIncentive {});
            }
            let mut incentive =
                Incentive::new(deps.storage, denom, schedule, &Timestamp::from_nanos(0))?;
            if let Some(coin) = incentive.distribute(&env.block.time) {
                STATE_MACHINE.distribute_rewards(deps.storage, &vec![coin])?;
            }
            incentive.save(deps.storage)?;

            Ok(Response::default())
        }
        ExecuteMsg::UpdateConfig(msg) => {
            ensure!(info.sender == config.owner, ContractError::Unauthorized {});
            config.apply_update(msg)?;
            config.save(deps.storage, deps.api)?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(match msg {
        QueryMsg::Config {} => to_json_binary(&ConfigResponse::from(config)),
        QueryMsg::PendingRewards { staker } => {
            to_json_binary(&query::pending_rewards(deps, env, &config, staker)?)
        }
        QueryMsg::StakeInfo { staker } => to_json_binary(&query::stake_info(deps, staker)?),
        QueryMsg::Weights { start_after, limit } => {
            to_json_binary(&query::weights(deps, start_after, limit)?)
        }
        QueryMsg::Incentives { start_after, limit } => {
            to_json_binary(&query::incentives(deps, start_after, limit)?)
        }
    }?)
}
