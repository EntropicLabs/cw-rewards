use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure_eq, to_json_binary, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use cw4::MemberDiff;
use kujira::{KujiraMsg, KujiraQuery};
use rewards_interfaces::{
    hooked_incentive::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg},
    RewardsMsg,
};
use rewards_logic::{incentive, RewardsSM};

use crate::{execute, query, Config, ContractError};

const CONTRACT_NAME: &str = "entropic/rewards";
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
        ExecuteMsg::MemberChangedHook(msg) => {
            ensure_eq!(info.sender, config.hook_src, ContractError::Unauthorized {});
            let mut attrs = vec![];
            for MemberDiff { key, new, .. } in msg.diffs {
                let weight = Uint128::from(new.unwrap_or_default());
                STATE_MACHINE.set_weight(deps.storage, &key, weight, false)?;
                attrs.push(("staker", key));
                attrs.push(("weight", weight.to_string()));
            }

            Ok(Response::default()
                .add_event(Event::new("rewards/hooked/update-weights").add_attributes(attrs)))
        }
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
                RewardsMsg::Stake(_) => Err(ContractError::Unauthorized {}),
                RewardsMsg::Unstake(_) => Err(ContractError::Unauthorized {}),
                RewardsMsg::ClaimRewards(msg) => execute::claim(deps, info, msg),
                RewardsMsg::DistributeRewards(msg) => execute::distribute(deps, info, config, msg),
            }
        }
        ExecuteMsg::UpdateConfig(msg) => {
            ensure_eq!(info.sender, config.owner, ContractError::Unauthorized {});
            config.apply_update(msg)?;
            config.save(deps.storage, deps.api)?;
            Ok(Response::default())
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<KujiraQuery>, _env: Env, msg: QueryMsg) -> Result<Binary, ContractError> {
    let config = Config::load(deps.storage)?;
    Ok(match msg {
        QueryMsg::Config {} => to_json_binary(&ConfigResponse::from(config)),
        QueryMsg::PendingRewards { staker } => {
            to_json_binary(&query::pending_rewards(deps, staker)?)
        }
        QueryMsg::StakeInfo { staker } => to_json_binary(&query::stake_info(deps, staker)?),
        QueryMsg::Incentives { start_after, limit } => {
            to_json_binary(&query::incentives(deps, start_after, limit)?)
        }
    }?)
}
