#![cfg(test)]
mod multi;

use cosmwasm_std::{coin, coins, Decimal, StdResult, Uint128};
use cw_multi_test::Executor;
use kujira::fee_address;
use kujira_rs_testing::mock::CustomApp;
use rewards_interfaces::{permissioned_incentive::*, *};

use crate::testing::multi::setup_env;

use self::multi::Addrs;

fn set_default_weights(app: &mut CustomApp, a: &Addrs) {
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::AdjustWeights {
            delta: vec![
                (a.user.clone(), 150u128.into()),
                (a.user2.clone(), 50u128.into()),
            ],
        },
        &[],
    )
    .unwrap();
}

#[test]
fn test_set_weights() {
    let (mut app, a) = setup_env();
    let res = app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::AdjustWeights {
            delta: vec![
                (a.user.clone(), 100u128.into()),
                (a.admin.clone(), 10u128.into()),
            ],
        },
        &[],
    );

    assert!(res.is_ok());

    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::from(100u128));

    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.admin.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::from(10u128));
}

#[test]
fn test_update_weights() {
    let (mut app, a) = setup_env();
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::AdjustWeights {
            delta: vec![(a.user.clone(), 100u128.into())],
        },
        &[],
    )
    .unwrap();

    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::from(100u128));

    let res = app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::AdjustWeights {
            delta: vec![(a.user.clone(), 10u128.into())],
        },
        &[],
    );

    assert!(res.is_ok());

    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::from(10u128));

    let res = app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::AdjustWeights {
            delta: vec![(a.user.clone(), 0u128.into())],
        },
        &[],
    );

    assert!(res.is_ok());

    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::zero());
}

#[test]
fn test_distribute_rewards() {
    let (mut app, a) = setup_env();
    set_default_weights(&mut app, &a);

    let res = app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    );
    assert!(res.is_ok());

    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(pending.rewards, vec![coin(750, "TOKEN")]);
}

#[test]
fn test_claim_rewards() {
    let (mut app, a) = setup_env();
    set_default_weights(&mut app, &a);

    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    )
    .unwrap();

    let res = app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(ClaimRewardsMsg { callback: None }.into()),
        &[],
    );
    assert!(res.is_ok());

    let user_balance = app.wrap().query_balance(&a.user, "TOKEN").unwrap();
    assert_eq!(
        user_balance.amount,
        Uint128::from(750u128 + 1_000_000_000_000u128)
    );
}

#[test]
fn test_claim_without_staking() {
    let (mut app, a) = setup_env();
    let res = app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(ClaimRewardsMsg { callback: None }.into()),
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn test_distribute_without_staking() {
    let (mut app, a) = setup_env();
    let res = app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    );
    assert!(res.is_err());
}

#[test]
fn test_distribute_no_rewards() {
    let (mut app, a) = setup_env();
    set_default_weights(&mut app, &a);

    let res = app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &[],
    );
    assert!(res.is_err());
}

#[test]
fn test_fees() {
    let (mut app, a) = setup_env();
    let config_update = ConfigUpdate {
        owner: None,
        fees: Some(vec![
            (Decimal::percent(10), fee_address()),
            (Decimal::percent(10), fee_address()),
        ]),
        whitelisted_rewards: None,
        incentive_crank_limit: None,
        incentive_min: None,
        incentive_fee: None,
    };
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::UpdateConfig(config_update),
        &[],
    )
    .unwrap();
    set_default_weights(&mut app, &a);

    // Simulating reward distribution
    let res = app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    );
    assert!(res.is_ok());

    // pending
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(pending.rewards, vec![coin(600, "TOKEN")]); // 800 * 0.75 = 600
}
