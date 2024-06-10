use cosmwasm_std::{ensure, Coin, DepsMut, MessageInfo, Response};
use cw_utils::one_coin;
use kujira::Denom;
use rewards_interfaces::{
    claiming::WhitelistedRewards, ClaimRewardsMsg, DistributeRewardsMsg, StakeMsg, UnstakeMsg,
};
use rewards_logic::util::{calculate_fee_msgs, calculate_fee_split};

use crate::{contract::STATE_MACHINE, Config, ContractError};

pub fn stake(
    deps: DepsMut,
    info: MessageInfo,
    config: Config,
    msg: StakeMsg,
) -> Result<Response, ContractError> {
    let received = one_coin(&info)?;
    ensure!(
        received.denom == config.stake_denom.as_ref(),
        ContractError::StakeNotWhitelisted {}
    );

    rewards_logic::execute::stake(
        STATE_MACHINE,
        deps.storage,
        received,
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
    rewards_logic::execute::unstake(
        STATE_MACHINE,
        deps.storage,
        &info.sender,
        &config.stake_denom,
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
    rewards_logic::execute::claim(
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
    if let WhitelistedRewards::Some(whitelist) = &config.whitelisted_rewards {
        for Coin { denom, .. } in info.funds.iter() {
            if !whitelist.contains(&Denom::from(denom)) {
                return Err(ContractError::RewardNotWhitelisted {});
            }
        }
    }

    // Fee split
    let mut rewards = info.funds;
    let fees = calculate_fee_split(&mut rewards, &config.fees);
    let msgs = calculate_fee_msgs(fees);

    rewards_logic::execute::distribute_rewards(
        STATE_MACHINE,
        deps.storage,
        info.sender,
        rewards,
        msg,
        "rewards/simple",
    )
    .map_or_else(|e| Err(e.into()), |res| Ok(res.add_messages(msgs)))
}
