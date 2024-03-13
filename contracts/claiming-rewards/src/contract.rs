use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, to_json_binary, wasm_execute, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg,
};
use cw2::set_contract_version;
use kujira::{KujiraMsg, KujiraQuery};
use rewards_interfaces::{
    claiming::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    ClaimRewardsMsg, PendingRewardsResponse, RewardsMsg,
};
use rewards_logic::RewardsSM;

use crate::{execute, query, Config, ContractError};

const CONTRACT_NAME: &str = "entropic/claiming-rewards";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STATE_MACHINE: RewardsSM = RewardsSM::new();

#[cw_serde]
pub struct MigrateMsg {}

#[entry_point]
pub fn migrate(_deps: DepsMut<KujiraQuery>, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut<KujiraQuery>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config::from(msg);
    config.save(deps.storage, deps.api)?;

    STATE_MACHINE.initialize(deps.storage)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<KujiraQuery>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<KujiraMsg>, ContractError> {
    let mut config = Config::load(deps.storage)?;
    match msg {
        ExecuteMsg::Rewards(msg) => {
            let pending: PendingRewardsResponse = deps.querier.query_wasm_smart(
                &config.underlying_rewards,
                &QueryMsg::PendingRewards {
                    staker: env.contract.address.clone(),
                },
            )?;
            let msgs = if !pending.rewards.is_empty()
                && !STATE_MACHINE
                    .total_staked
                    .may_load(deps.storage)?
                    .unwrap_or_default()
                    .is_zero()
            {
                STATE_MACHINE.distribute_rewards(deps.storage, &pending.rewards)?;
                vec![SubMsg::new(wasm_execute(
                    &config.underlying_rewards,
                    &ExecuteMsg::Rewards(ClaimRewardsMsg { callback: None }.into()),
                    vec![],
                )?)]
            } else {
                vec![]
            };

            let mut res = match msg {
                RewardsMsg::Stake(msg) => execute::stake(deps, info, config, msg),
                RewardsMsg::Unstake(msg) => execute::unstake(deps, info, config, msg),
                RewardsMsg::ClaimRewards(msg) => execute::claim(deps, info, msg),
                RewardsMsg::DistributeRewards(msg) => execute::distribute(deps, info, config, msg),
            }?;
            res.messages = [msgs, res.messages].concat();

            Ok(res)
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
pub fn query(deps: Deps<KujiraQuery>, env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(match msg {
        QueryMsg::Config {} => to_json_binary(&ConfigResponse::from(config)),
        QueryMsg::PendingRewards { staker } => {
            to_json_binary(&query::pending_rewards(deps, env, &config, staker)?)
        }
        QueryMsg::StakeInfo { staker } => to_json_binary(&query::stake_info(deps, staker)?),
    }?)
}
