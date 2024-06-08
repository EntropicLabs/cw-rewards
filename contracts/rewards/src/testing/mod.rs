#![cfg(test)]
use std::marker::PhantomData;

use cosmwasm_std::{
    coins,
    testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage},
    BankMsg, CosmosMsg, Decimal, Decimal256, Env, MessageInfo, OwnedDeps, Uint128, Uint256,
};
use kujira::{fee_address, KujiraQuery};
use rewards_interfaces::{simple::*, *};
use rewards_logic::state_machine::RewardInfo;

use crate::{
    contract::instantiate,
    contract::STATE_MACHINE,
    execute::{claim, distribute, stake, unstake},
    Config,
};

type OwnedDepsType = OwnedDeps<MockStorage, MockApi, MockQuerier, KujiraQuery>;

pub fn mock_dependencies() -> OwnedDepsType {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: MockQuerier::default(),
        custom_query_type: PhantomData,
    }
}

fn setup_contract() -> (OwnedDepsType, Env, MessageInfo) {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let sender = deps.api.addr_make("sender");
    let info = message_info(&sender, &coins(100, "tokens"));

    let instantiate_msg = InstantiateMsg {
        owner: info.sender.clone(),
        stake_denom: "tokens".into(),
        whitelisted_rewards: WhitelistedRewards::All,
        fees: vec![],
    };

    instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();

    (deps, env, info)
}

#[test]
fn test_stake() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };

    stake(deps.as_mut(), info.clone(), config, stake_msg).unwrap();

    let total_staked = STATE_MACHINE.total_staked.load(&deps.storage).unwrap();
    assert_eq!(total_staked, Uint128::from(100u128));

    let user_stakes = STATE_MACHINE
        .user_weights
        .load(&deps.storage, &info.sender.to_string())
        .unwrap();
    assert_eq!(user_stakes, Uint128::from(100u128));
}

#[test]
fn test_unstake() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // First stake some tokens
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    // Now unstake
    let unstake_msg = UnstakeMsg {
        amount: Uint128::from(50u128),
        withdraw_rewards: false,
        callback: None,
    };
    unstake(deps.as_mut(), info.clone(), config, unstake_msg).unwrap();

    let total_staked = STATE_MACHINE.total_staked.load(&deps.storage).unwrap();
    assert_eq!(total_staked, Uint128::from(50u128));

    let user_stakes = STATE_MACHINE
        .user_weights
        .load(&deps.storage, &info.sender.to_string())
        .unwrap();
    assert_eq!(user_stakes, Uint128::from(50u128));
}

#[test]
fn test_claim_rewards() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // First stake some tokens
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    // Simulating reward distribution
    let distribute_msg = DistributeRewardsMsg { callback: None };
    distribute(deps.as_mut(), info.clone(), config, distribute_msg).unwrap();

    let claim_msg = ClaimRewardsMsg { callback: None };
    claim(deps.as_mut(), info.clone(), claim_msg).unwrap();

    // Check rewards for the staker
    let key = (&info.sender.to_string(), "tokens");
    let reward_info: RewardInfo = STATE_MACHINE.user_rewards.load(&deps.storage, key).unwrap();
    assert_eq!(reward_info.accrued, Uint128::zero());
}

#[test]
fn test_distribute_rewards() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // First stake some tokens
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    let distribute_msg = DistributeRewardsMsg { callback: None };
    distribute(deps.as_mut(), info, config, distribute_msg).unwrap();

    // Check the global index
    let global_index = STATE_MACHINE
        .global_indices
        .load(&deps.storage, "tokens")
        .unwrap();
    assert_eq!(
        global_index,
        Decimal256::from_ratio(Uint256::from(100u128), Uint256::from(100u128))
    );
}

#[test]
fn test_stake_zero() {
    let (mut deps, _env, _info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // Stake with zero amount
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };

    let info_zero = message_info(&deps.api.addr_make("sender"), &[]);
    let res = stake(deps.as_mut(), info_zero.clone(), config, stake_msg);
    assert!(res.is_err()); // Expect an error

    let total_staked = STATE_MACHINE.total_staked.load(&deps.storage).unwrap();
    assert_eq!(total_staked, Uint128::zero());

    let user_stakes = STATE_MACHINE
        .user_weights
        .may_load(&deps.storage, &info_zero.sender.to_string())
        .unwrap();
    assert_eq!(user_stakes, None);
}

#[test]
fn test_unstake_more_than_staked() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // First stake some tokens
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    // Now try to unstake more than the staked amount
    let unstake_msg = UnstakeMsg {
        amount: Uint128::from(150u128), // More than staked
        withdraw_rewards: false,
        callback: None,
    };
    let result = unstake(deps.as_mut(), info, config, unstake_msg);
    assert!(result.is_err()); // Expect an error
}

#[test]
fn test_claim_without_staking() {
    let (mut deps, _env, info) = setup_contract();

    // Claim without staking
    let claim_msg = ClaimRewardsMsg { callback: None };
    let result = claim(deps.as_mut(), info, claim_msg);
    assert!(result.is_err()); // Expect an error
}

#[test]
fn test_distribute_without_staking() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    let distribute_msg = DistributeRewardsMsg { callback: None };
    let res = distribute(deps.as_mut(), info, config, distribute_msg);
    assert!(res.is_err()); // Expect an error
}

#[test]
fn test_unstake_zero() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // First stake some tokens
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    // Now unstake zero amount
    let unstake_msg = UnstakeMsg {
        amount: Uint128::zero(),
        withdraw_rewards: false,
        callback: None,
    };
    let result = unstake(deps.as_mut(), info, config, unstake_msg);
    assert!(result.is_err()); // Expect an error
}

#[test]
fn test_multiple_stakes_same_user() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // Stake multiple times by the same user
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };

    stake(
        deps.as_mut(),
        info.clone(),
        config.clone(),
        stake_msg.clone(),
    )
    .unwrap();
    stake(deps.as_mut(), info.clone(), config, stake_msg).unwrap();

    let total_staked = STATE_MACHINE.total_staked.load(&deps.storage).unwrap();
    assert_eq!(total_staked, Uint128::from(200u128)); // 100 + 100

    let user_stakes = STATE_MACHINE
        .user_weights
        .load(&deps.storage, &info.sender.to_string())
        .unwrap();
    assert_eq!(user_stakes, Uint128::from(200u128)); // 100 + 100
}

#[test]
fn test_multiple_stakes_multiple_users() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };

    // Stake by first user
    stake(
        deps.as_mut(),
        info.clone(),
        config.clone(),
        stake_msg.clone(),
    )
    .unwrap();

    // Stake by second user
    let sender2 = deps.api.addr_make("another_sender");
    let info2 = message_info(&sender2, &coins(200, "tokens"));
    stake(deps.as_mut(), info2.clone(), config, stake_msg).unwrap();

    let total_staked = STATE_MACHINE.total_staked.load(&deps.storage).unwrap();
    assert_eq!(total_staked, Uint128::from(300u128)); // 100 + 200

    let user_stakes1 = STATE_MACHINE
        .user_weights
        .load(&deps.storage, &info.sender.to_string())
        .unwrap();
    assert_eq!(user_stakes1, Uint128::from(100u128));

    let user_stakes2 = STATE_MACHINE
        .user_weights
        .load(&deps.storage, &info2.sender.to_string())
        .unwrap();
    assert_eq!(user_stakes2, Uint128::from(200u128));
}

#[test]
fn test_unstake_all() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    // Unstake all
    let unstake_msg = UnstakeMsg {
        amount: Uint128::from(100u128),
        withdraw_rewards: false,
        callback: None,
    };
    unstake(deps.as_mut(), info.clone(), config, unstake_msg).unwrap();

    let total_staked = STATE_MACHINE.total_staked.load(&deps.storage).unwrap();
    assert_eq!(total_staked, Uint128::zero());

    let user_stakes = STATE_MACHINE
        .user_weights
        .may_load(&deps.storage, &info.sender.to_string())
        .unwrap();
    assert_eq!(user_stakes, None);
}

#[test]
fn test_distribute_no_rewards() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    //First stake some tokens
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info, config.clone(), stake_msg).unwrap();

    let distribute_msg = DistributeRewardsMsg { callback: None };
    let info_zero = message_info(&deps.api.addr_make("sender"), &[]);
    let res = distribute(deps.as_mut(), info_zero, config, distribute_msg);
    assert!(res.is_err()); // Expect an error
}

#[test]
fn test_claim_after_unstake_all() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // First stake and then unstake all
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    let unstake_msg = UnstakeMsg {
        amount: Uint128::from(100u128),
        withdraw_rewards: false,
        callback: None,
    };
    unstake(deps.as_mut(), info.clone(), config, unstake_msg).unwrap();

    // Claim after unstaking all
    let claim_msg = ClaimRewardsMsg { callback: None };
    let result = claim(deps.as_mut(), info, claim_msg);
    assert!(result.is_err()); // Expect an error
}

#[test]
fn test_claim_after_stake_distribute_unstake() {
    let (mut deps, _env, info) = setup_contract();
    let config = Config::load(deps.as_mut().storage).unwrap();
    let stake_denom = config.stake_denom.clone();

    // First stake some tokens
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    // Simulating reward distribution
    let distribute_msg = DistributeRewardsMsg { callback: None };
    distribute(deps.as_mut(), info.clone(), config.clone(), distribute_msg).unwrap();

    // Unstake all
    let unstake_msg = UnstakeMsg {
        amount: Uint128::from(100u128),
        withdraw_rewards: false,
        callback: None,
    };
    unstake(deps.as_mut(), info.clone(), config, unstake_msg).unwrap();

    // Check rewards for the staker
    let key = (&info.sender.to_string(), stake_denom.as_ref());
    let reward_info: RewardInfo = STATE_MACHINE.user_rewards.load(&deps.storage, key).unwrap();
    assert_eq!(reward_info.accrued, Uint128::from(100u128));

    // Claim after staking and distributing rewards
    let claim_msg = ClaimRewardsMsg { callback: None };
    claim(deps.as_mut(), info, claim_msg).unwrap();

    // Check rewards for the staker
    let reward_info: Option<RewardInfo> = STATE_MACHINE
        .user_rewards
        .may_load(&deps.storage, key)
        .unwrap();
    assert_eq!(reward_info, None);
}

#[test]
fn test_fees() {
    let (mut deps, _env, info) = setup_contract();
    let config_update = ConfigUpdate {
        owner: None,
        stake_denom: None,
        fees: Some(vec![
            (Decimal::percent(10), fee_address()),
            (Decimal::percent(10), fee_address()),
        ]),
        whitelisted_rewards: None,
    };
    crate::contract::execute(
        deps.as_mut(),
        mock_env(),
        info.clone(),
        ExecuteMsg::UpdateConfig(config_update),
    )
    .unwrap();
    let config = Config::load(deps.as_mut().storage).unwrap();

    // First stake some tokens
    let stake_msg = StakeMsg {
        withdraw_rewards: false,
        callback: None,
    };
    stake(deps.as_mut(), info.clone(), config.clone(), stake_msg).unwrap();

    // Simulating reward distribution
    let distribute_msg = DistributeRewardsMsg { callback: None };
    let res = distribute(deps.as_mut(), info.clone(), config.clone(), distribute_msg).unwrap();
    assert_eq!(res.messages.len(), 2);
    assert_eq!(
        res.messages[0].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: fee_address().to_string(),
            amount: coins(10, "tokens")
        })
    );
    assert_eq!(
        res.messages[1].msg,
        CosmosMsg::Bank(BankMsg::Send {
            to_address: fee_address().to_string(),
            amount: coins(10, "tokens")
        })
    );
}
