use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure_eq, to_json_binary, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
    Uint128,
};
use cw2::set_contract_version;
use cw4::MemberDiff;

use rewards_interfaces::{
    hooked::{
        ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, StakeChangedHookMsg, WeightsSource,
    },
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
    mod weights {
        use cosmwasm_schema::cw_serde;
        use cosmwasm_std::Uint128;

        #[cw_serde]
        pub enum DaoDaoQueryMsg {
            ListStakers {
                start_after: Option<String>,
                limit: Option<u32>,
            },
        }

        #[cw_serde]
        pub struct ListStakersResponse {
            pub stakers: Vec<StakerBalanceResponse>,
        }

        #[cw_serde]
        pub struct StakerBalanceResponse {
            pub address: String,
            pub balance: Uint128,
        }
    }
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let weight_src = msg.initialize_weights_from.clone();
    let config = Config::from(msg);
    config.save(deps.storage, deps.api)?;

    STATE_MACHINE.initialize(deps.storage)?;

    if let Some(src) = weight_src {
        match src {
            WeightsSource::DAODAO { staking } => {
                let mut weights: Vec<weights::StakerBalanceResponse> = vec![];
                let mut list_stakers: weights::ListStakersResponse;
                loop {
                    let start_after = weights.last().map(|w| w.address.clone());
                    list_stakers = deps.querier.query_wasm_smart(
                        &staking,
                        &weights::DaoDaoQueryMsg::ListStakers {
                            start_after,
                            limit: Some(30), // Max is 30
                        },
                    )?;
                    if list_stakers.stakers.is_empty() {
                        break;
                    }

                    weights.extend(list_stakers.stakers);
                }

                for staker in weights {
                    STATE_MACHINE.set_weight(
                        deps.storage,
                        &staker.address,
                        staker.balance,
                        false,
                    )?;
                }
            }
        }
    }

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
        // Weight change hook from DAODAO
        ExecuteMsg::StakeChangeHook(msg) => {
            ensure_eq!(info.sender, config.hook_src, ContractError::Unauthorized {});
            match msg {
                StakeChangedHookMsg::Stake { addr, amount } => {
                    STATE_MACHINE.increase_weight(
                        deps.storage,
                        &addr.to_string(),
                        amount,
                        false,
                    )?;
                }
                StakeChangedHookMsg::Unstake { addr, amount } => {
                    STATE_MACHINE.decrease_weight(
                        deps.storage,
                        &addr.to_string(),
                        amount,
                        false,
                    )?;
                }
            };

            Ok(Response::default().add_event(Event::new("rewards/hooked/update-weights")))
        }
        // Weight change hook from CW4
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
        ExecuteMsg::Rewards(RewardsMsg::Stake(_) | RewardsMsg::Unstake(_)) => {
            Err(ContractError::Unauthorized {})
        }
        ExecuteMsg::Rewards(RewardsMsg::ClaimRewards(msg)) => execute::claim(deps, info, msg),
        ExecuteMsg::Rewards(RewardsMsg::DistributeRewards(msg)) => {
            execute::distribute(deps, info, config, msg)
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
