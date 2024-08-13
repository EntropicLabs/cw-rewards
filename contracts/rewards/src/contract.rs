#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, ensure_eq, to_json_binary, wasm_execute, BankMsg, Binary, Deps, DepsMut, Env, Event,
    MessageInfo, Response, SubMsg, Timestamp,
};
use cw2::set_contract_version;
use cw4::MemberDiff;
use cw_utils::{one_coin, NativeBalance};

use crate::migration::MigrateMsg;
use crate::msg::*;
use cw_rewards_logic::{incentive, inflation, RewardsSM};
use cw_rewards_logic::{ClaimRewardsMsg, PendingRewardsResponse, RewardsMsg};

use crate::{execute, query, Config, ContractError};

pub const CONTRACT_NAME: &str = "entropic/cw-rewards";
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const STATE_MACHINE: RewardsSM = RewardsSM::new();

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    crate::migration::do_migrate(deps, env, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
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
    let config = Config::from(msg);
    config.save(deps.storage, deps.api)?;

    STATE_MACHINE.initialize(deps.storage)?;

    if let StakingConfig::DaoDaoHook { daodao_addr } = config.staking_module {
        let mut weights: Vec<weights::StakerBalanceResponse> = vec![];
        let mut list_stakers: weights::ListStakersResponse;
        loop {
            let start_after = weights.last().map(|w| w.address.clone());
            list_stakers = deps.querier.query_wasm_smart(
                &daodao_addr,
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
            STATE_MACHINE.set_weight(deps.storage, &staker.address, staker.balance, false)?;
        }
    }

    if config.inflation_module.is_some() {
        inflation::LAST_INFLATION_UPDATE.save(deps.storage, &env.block.time)?;
    }

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

            if let Some(inflation) = &config.inflation_module {
                inflation::crank(
                    deps.storage,
                    STATE_MACHINE,
                    &inflation.rate_per_year,
                    &env.block.time,
                )?;
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

            let mut incentive = incentive::Incentive::new(
                deps.storage,
                denom,
                schedule,
                &Timestamp::from_nanos(0),
            )?;
            if let Some(coin) = incentive.distribute(&env.block.time) {
                STATE_MACHINE.distribute_rewards(deps.storage, &vec![coin])?;
            }
            incentive.save(deps.storage)?;

            Ok(Response::default())
        }
        ExecuteMsg::FundInflation {} => {
            ensure!(info.sender == config.owner, ContractError::Unauthorized {});
            ensure!(
                config.inflation_module.is_some(),
                ContractError::InflationNotEnabled {}
            );
            let funds = one_coin(&info)?;
            inflation::fund(deps.storage, funds)?;

            Ok(Response::default())
        }
        ExecuteMsg::WithdrawInflation { amount } => {
            ensure!(info.sender == config.owner, ContractError::Unauthorized {});
            ensure!(
                config.inflation_module.is_some(),
                ContractError::InflationNotEnabled {}
            );
            let withdraw_coin = inflation::withdraw(
                deps.storage,
                &STATE_MACHINE,
                &config.inflation_module.unwrap().rate_per_year,
                &env.block.time,
                amount,
            )?;
            let withdraw_msg = BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![withdraw_coin],
            };

            Ok(Response::default().add_message(withdraw_msg))
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
            // If enabling inflation, set the last update time to now.
            if let (None, Some(ModuleUpdate { update: Some(_) })) =
                (&config.inflation_module, &msg.inflation_cfg)
            {
                inflation::LAST_INFLATION_UPDATE.save(deps.storage, &env.block.time)?;
            }

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
        QueryMsg::Config {} => to_json_binary(&config),
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
        QueryMsg::Inflation {} => to_json_binary(&query::inflation(deps, env, &config)?),
    }?)
}
