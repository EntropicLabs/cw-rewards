use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, ensure_eq, to_json_binary, wasm_execute, Binary, Deps, DepsMut, Env, Event,
    MessageInfo, Response, StdResult, SubMsg, Timestamp,
};
use cw2::set_contract_version;
use cw4::MemberDiff;
use cw_utils::NativeBalance;

use rewards_interfaces::{
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, StakeChangedHookMsg},
    modules::{StakingConfig, Whitelist},
    ClaimRewardsMsg, PendingRewardsResponse, RewardsMsg,
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
            let zero_staked = STATE_MACHINE.total_staked(deps.storage)?.is_zero();
            if config.incentive_module.is_some() && !zero_staked {
                let incentive_cfg = config.incentive_module.as_ref().unwrap();
                incentive::distribute_lri(
                    deps.storage,
                    incentive_cfg.crank_limit,
                    STATE_MACHINE,
                    &env.block.time,
                )?;
            }

            let mut claim_underlying_msgs = vec![];
            if let Some(underlying_rewards) = &config.underlying_rewards_module {
                let pending: PendingRewardsResponse = deps.querier.query_wasm_smart(
                    &underlying_rewards.underlying_rewards_contract,
                    &QueryMsg::PendingRewards {
                        staker: env.contract.address.clone(),
                    },
                )?;
                if !pending.rewards.is_empty() && !zero_staked {
                    STATE_MACHINE.distribute_rewards(deps.storage, &pending.rewards)?;
                    claim_underlying_msgs.push(wasm_execute(
                        &underlying_rewards.underlying_rewards_contract,
                        &ExecuteMsg::Rewards(ClaimRewardsMsg { callback: None }.into()),
                        vec![],
                    )?);
                }
            }

            let res = match msg {
                RewardsMsg::Stake(msg) => execute::stake(deps, info, config, msg),
                RewardsMsg::Unstake(msg) => execute::unstake(deps, info, config, msg),
                RewardsMsg::ClaimRewards(msg) => execute::claim(deps, info, msg),
                RewardsMsg::DistributeRewards(msg) => execute::distribute(deps, info, config, msg),
            };

            res.map(|mut res| {
                res.messages = [
                    claim_underlying_msgs.into_iter().map(SubMsg::new).collect(),
                    res.messages,
                ]
                .concat();
                res
            })
        }

        // Weight change hook from DAODAO
        ExecuteMsg::StakeChangeHook(msg) => {
            let src_addr = match config.staking_module {
                StakingConfig::DaoDaoHook { daodao_addr } => daodao_addr,
                _ => {
                    return Err(ContractError::InvalidStakingConfig(
                        "DAODAO hook",
                        config.staking_module,
                    ))
                }
            };
            ensure_eq!(info.sender, src_addr, ContractError::Unauthorized {});
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

            Ok(Response::default().add_event(Event::new("rewards/update-weight-hook")))
        }
        // Weight change hook from CW4
        ExecuteMsg::MemberChangedHook(msg) => {
            let src_addr = match config.staking_module {
                StakingConfig::Cw4Hook { cw4_addr } => cw4_addr,
                _ => {
                    return Err(ContractError::InvalidStakingConfig(
                        "CW4 hook",
                        config.staking_module,
                    ))
                }
            };
            ensure_eq!(info.sender, src_addr, ContractError::Unauthorized {});
            let mut attrs = vec![];
            for MemberDiff { key, new, .. } in msg.diffs {
                let weight = new.unwrap_or_default().into();
                STATE_MACHINE.set_weight(deps.storage, &key, weight, false)?;
                attrs.push(("staker", key));
                attrs.push(("weight", weight.to_string()));
            }

            Ok(Response::default()
                .add_event(Event::new("rewards/update-weights-hook").add_attributes(attrs)))
        }
        ExecuteMsg::AddIncentive { denom, schedule } => {
            if config.incentive_module.is_none() {
                return Err(ContractError::IncentivesNotEnabled {});
            }
            let incentive_cfg = config.incentive_module.as_ref().unwrap();

            let mut sent = NativeBalance(info.funds);
            if let Some(fee) = incentive_cfg.fee.clone() {
                sent = (sent - fee).map_err(|_| ContractError::InvalidIncentive {})?;
            }
            let sent = sent.into_vec();

            if sent.len() != 1
                || sent[0].amount < incentive_cfg.min_size
                || sent[0].denom != denom.as_ref()
            {
                return Err(ContractError::InvalidIncentive {});
            }

            if let Whitelist::Some(denoms) = &incentive_cfg.whitelisted_denoms {
                ensure!(denoms.contains(&denom), ContractError::InvalidIncentive {});
            }

            let mut incentive =
                Incentive::new(deps.storage, denom, schedule, &Timestamp::from_nanos(0))?;
            if let Some(coin) = incentive.distribute(&env.block.time) {
                STATE_MACHINE.distribute_rewards(deps.storage, &vec![coin])?;
            }
            incentive.save(deps.storage)?;

            Ok(Response::default())
        }
        ExecuteMsg::AdjustWeights { delta } => {
            ensure!(
                matches!(config.staking_module, StakingConfig::Permissioned {}),
                ContractError::InvalidStakingConfig("AdjustWeights", config.staking_module)
            );
            ensure!(info.sender == config.owner, ContractError::Unauthorized {});
            for (addr, weight) in delta {
                STATE_MACHINE.set_weight(deps.storage, &addr.to_string(), weight, false)?;
            }

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
