use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use rewards_interfaces::{
    simple::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    RewardsMsg,
};
use rewards_logic::RewardsSM;

use crate::{execute, query, Config, ContractError};

const CONTRACT_NAME: &str = "entropic/rewards";
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
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut config = Config::load(deps.storage)?;
    match msg {
        ExecuteMsg::Rewards(RewardsMsg::Stake(msg)) => execute::stake(deps, info, config, msg),
        ExecuteMsg::Rewards(RewardsMsg::Unstake(msg)) => execute::unstake(deps, info, config, msg),
        ExecuteMsg::Rewards(RewardsMsg::ClaimRewards(msg)) => execute::claim(deps, info, msg),
        ExecuteMsg::Rewards(RewardsMsg::DistributeRewards(msg)) => {
            execute::distribute(deps, info, config, msg)
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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(match msg {
        QueryMsg::Config {} => to_json_binary(&ConfigResponse::from(config)),
        QueryMsg::PendingRewards { staker } => {
            to_json_binary(&query::pending_rewards(deps, staker)?)
        }
        QueryMsg::StakeInfo { staker } => to_json_binary(&query::stake_info(deps, staker)?),
    }?)
}
