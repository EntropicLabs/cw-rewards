use cosmwasm_std::{
    ensure, Addr, BankMsg, Coin, CosmosMsg, CustomMsg, Empty, Event, Response, Storage,
};
use cw_utils::NativeBalance;
use kujira::Denom;
use rewards_interfaces::{
    ClaimRewardsMsg, DistributeRewardsMsg, RewardsError, StakeMsg, UnstakeMsg,
};

use super::RewardsSM;

pub fn stake<T: CustomMsg>(
    sm: RewardsSM,
    storage: &mut dyn Storage,
    stake_coin: Coin,
    user: &Addr,
    msg: StakeMsg,
    namespace: &str,
) -> Result<Response<T>, RewardsError> {
    let coins = sm.increase_weight(
        storage,
        &user.to_string(),
        stake_coin.amount,
        msg.withdraw_rewards,
    )?;

    let mut msgs: Vec<CosmosMsg<_>> = vec![];
    match (msg.callback, msg.withdraw_rewards && !coins.is_empty()) {
        (None, false) => {}
        (None, true) => {
            msgs.push(
                BankMsg::Send {
                    to_address: user.to_string(),
                    amount: coins,
                }
                .into(),
            );
        }
        (Some(cb), false) => {
            msgs.push(cb.to_message(user, Empty {}, vec![])?);
        }
        (Some(cb), true) => {
            msgs.push(cb.to_message(user, Empty {}, coins)?);
        }
    }

    let event = Event::new(format!("{namespace}/rewards/stake")).add_attributes(vec![
        ("action", "rewards/stake"),
        ("staker", user.as_str()),
        ("amount", &stake_coin.amount.to_string()),
        ("denom", &stake_coin.denom),
        ("withdraw_rewards", &msg.withdraw_rewards.to_string()),
    ]);

    Ok(Response::new().add_messages(msgs).add_event(event))
}

pub fn unstake<T: CustomMsg>(
    sm: RewardsSM,
    storage: &mut dyn Storage,
    user: &Addr,
    stake_denom: &Denom,
    msg: UnstakeMsg,
    namespace: &str,
) -> Result<Response<T>, RewardsError> {
    ensure!(!msg.amount.is_zero(), RewardsError::ZeroUnstake {});

    let mut coins =
        sm.decrease_weight(storage, &user.to_string(), msg.amount, msg.withdraw_rewards)?;

    // add the stake denom to the coins
    if msg.withdraw_rewards {
        coins = (NativeBalance(coins) + stake_denom.coin(&msg.amount)).into_vec();
    } else {
        coins = stake_denom.coins(&msg.amount);
    }

    let return_msg = match msg.callback {
        None => BankMsg::Send {
            to_address: user.to_string(),
            amount: coins,
        }
        .into(),
        Some(cb) => cb.to_message(user, Empty {}, coins)?,
    };

    let event = Event::new(format!("{namespace}/rewards/unstake")).add_attributes(vec![
        ("action", "rewards/unstake"),
        ("staker", user.as_str()),
        ("amount", &msg.amount.to_string()),
        ("denom", stake_denom.as_ref()),
        ("withdraw_rewards", &msg.withdraw_rewards.to_string()),
    ]);

    Ok(Response::new().add_message(return_msg).add_event(event))
}

pub fn claim<T: CustomMsg>(
    sm: RewardsSM,
    storage: &mut dyn Storage,
    user: &Addr,
    msg: ClaimRewardsMsg,
    namespace: &str,
) -> Result<Response<T>, RewardsError> {
    let coins = sm.claim_accrued(storage, &user.to_string())?;
    ensure!(!coins.is_empty(), RewardsError::NoRewardsToClaim {});

    let return_msg = match msg.callback {
        None => BankMsg::Send {
            to_address: user.to_string(),
            amount: coins.clone(),
        }
        .into(),
        Some(cb) => cb.to_message(user, Empty {}, coins.clone())?,
    };

    let event = Event::new(format!("{namespace}/rewards/claim"))
        .add_attributes(vec![("action", "rewards/claim"), ("staker", user.as_str())]);

    Ok(Response::new().add_message(return_msg).add_event(event))
}

pub fn distribute_rewards<T: CustomMsg>(
    sm: RewardsSM,
    storage: &mut dyn Storage,
    sender: Addr,
    rewards: Vec<Coin>,
    msg: DistributeRewardsMsg,
    namespace: &str,
) -> Result<Response<T>, RewardsError> {
    sm.distribute_rewards(storage, &rewards)?;

    let event = Event::new(format!("{namespace}/rewards/distribute")).add_attributes(vec![
        ("action", "rewards/distribute"),
        ("sender", sender.as_str()),
    ]);

    if let Some(cb) = msg.callback {
        let return_msg = cb.to_message(&sender, Empty {}, vec![])?;
        Ok(Response::new().add_message(return_msg).add_event(event))
    } else {
        Ok(Response::new().add_event(event))
    }
}
