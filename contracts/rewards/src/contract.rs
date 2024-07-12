use cosmwasm_schema::cw_serde;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    ensure, ensure_eq, to_json_binary, wasm_execute, Binary, Deps, DepsMut, Env, Event,
    MessageInfo, Response, StdError, SubMsg, Timestamp,
};
use cw2::set_contract_version;
use cw4::MemberDiff;
use cw_utils::NativeBalance;

use rewards_interfaces::{
    modules::{DistributionConfig, IncentiveConfig, StakingConfig, UnderlyingConfig, Whitelist},
    msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, StakeChangedHookMsg},
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
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    mod old {
        use cosmwasm_schema::cw_serde;
        use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
        use cw_storage_plus::Item;
        use kujira::Denom;
        use rewards_interfaces::modules::Whitelist;

        #[cw_serde]
        pub struct OldConfig {
            pub owner: Addr,
            pub whitelisted_rewards: Whitelist,
            pub fees: Vec<(Decimal, Addr)>,
            pub stake_denom: Option<Denom>,
            #[serde(flatten)]
            pub incentive: Option<OldIncentiveConfig>,
            pub hook_src: Option<Addr>,
            pub underlying_rewards: Option<Addr>,
        }

        #[cw_serde]
        pub struct OldIncentiveConfig {
            pub incentive_crank_limit: usize,
            pub incentive_min: Uint128,
            pub incentive_fee: Coin,
        }

        pub const CONFIG: Item<OldConfig> = Item::new("config");
    }
    let old_cfg = old::CONFIG.load(deps.storage)?;
    let staking_cfg = match (old_cfg.hook_src, old_cfg.stake_denom) {
        (None, Some(stake_denom)) => StakingConfig::NativeToken {
            denom: stake_denom.to_string(),
        },
        (Some(hook_src), None) => {
            let cw2_info = cw2::query_contract_info(&deps.querier, &hook_src)?;
            if cw2_info.contract == "crates.io:cw4-stake" {
                StakingConfig::Cw4Hook { cw4_addr: hook_src }
            } else if cw2_info.contract == "crates.io:dao-voting-token-staked" {
                StakingConfig::DaoDaoHook {
                    daodao_addr: hook_src,
                }
            } else {
                return Err(StdError::generic_err("Invalid old staking config").into());
            }
        }
        (None, None) => StakingConfig::Permissioned {},
        _ => return Err(StdError::generic_err("Invalid old staking config").into()),
    };
    let incentive_cfg = old_cfg.incentive.map(|o| IncentiveConfig {
        crank_limit: o.incentive_crank_limit,
        min_size: o.incentive_min,
        fee: Some(o.incentive_fee),
        whitelisted_denoms: Whitelist::All,
    });
    let distribution_cfg = DistributionConfig {
        whitelisted_denoms: old_cfg.whitelisted_rewards,
        fees: old_cfg.fees,
    };
    let underlying_cfg = old_cfg
        .underlying_rewards
        .map(|underlying| UnderlyingConfig {
            underlying_rewards_contract: underlying,
        });

    let new_cfg = Config {
        owner: old_cfg.owner,
        staking_module: staking_cfg,
        incentive_module: incentive_cfg,
        distribution_module: Some(distribution_cfg),
        underlying_rewards_module: underlying_cfg,
    };
    new_cfg.save(deps.storage, deps.api)?;
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
