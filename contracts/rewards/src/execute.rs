use cosmwasm_std::{coin, Coin, DepsMut, MessageInfo, Response};
use cw_rewards_logic::util::{calculate_fee_msgs, calculate_fee_split};
use cw_rewards_logic::{ClaimRewardsMsg, DistributeRewardsMsg, StakeMsg, UnstakeMsg};
use cw_utils::must_pay;

use crate::msg::{StakingConfig, Whitelist};
use crate::{contract::STATE_MACHINE, Config, ContractError};

pub fn stake(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
    msg: StakeMsg,
) -> Result<Response, ContractError> {
    let stake_denom = match config.staking_module {
        StakingConfig::NativeToken { denom } => denom,
        _ => {
            return Err(ContractError::InvalidStakingConfig(
                "NativeToken",
                config.staking_module,
            ))
        }
    };
    let received = must_pay(&info, &stake_denom)?;

    cw_rewards_logic::execute::stake(
        STATE_MACHINE,
        deps.storage,
        coin(received.u128(), &stake_denom),
        &info.sender,
        msg,
        "rewards/simple",
    )
    .map_err(ContractError::from)
}

pub fn unstake(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
    msg: UnstakeMsg,
) -> Result<Response, ContractError> {
    let stake_denom = match config.staking_module {
        StakingConfig::NativeToken { denom } => denom,
        _ => {
            return Err(ContractError::InvalidStakingConfig(
                "NativeToken",
                config.staking_module,
            ))
        }
    };

    cw_rewards_logic::execute::unstake(
        STATE_MACHINE,
        deps.storage,
        &info.sender,
        &stake_denom,
        msg,
        "rewards/simple",
    )
    .map_err(ContractError::from)
}

pub fn claim(
    deps: DepsMut,
    info: MessageInfo,
    msg: ClaimRewardsMsg,
) -> Result<Response, ContractError> {
    cw_rewards_logic::execute::claim(
        STATE_MACHINE,
        deps.storage,
        &info.sender,
        msg,
        "rewards/simple",
    )
    .map_err(ContractError::from)
}

pub fn distribute(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
    msg: DistributeRewardsMsg,
) -> Result<Response, ContractError> {
    if info.funds.is_empty() {
        return Err(ContractError::ZeroRewards {});
    }

    let distribution_cfg = match config.distribution_module {
        Some(cfg) => cfg,
        None => return Err(ContractError::DistributionNotEnabled {}),
    };

    if let Whitelist::Some(whitelist) = &distribution_cfg.whitelisted_denoms {
        for Coin { denom, .. } in info.funds.iter() {
            if !whitelist.contains(denom) {
                return Err(ContractError::RewardNotWhitelisted {});
            }
        }
    }

    // Fee split
    let mut rewards = info.funds;
    let fees = calculate_fee_split(&mut rewards, &distribution_cfg.fees);
    let msgs = calculate_fee_msgs(fees);

    cw_rewards_logic::execute::distribute_rewards(
        STATE_MACHINE,
        deps.storage,
        info.sender,
        rewards,
        msg,
        "rewards/simple",
    )
    .map_or_else(|e| Err(e.into()), |res| Ok(res.add_messages(msgs)))
}
