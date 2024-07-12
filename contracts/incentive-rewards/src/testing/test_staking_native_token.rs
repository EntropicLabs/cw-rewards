use cosmwasm_std::{coin, Uint128};
use cw_multi_test::Executor;
use rewards_interfaces::{msg::*, *};

use super::test_helpers::setup_test_env;

#[test]
fn test_stake_native_token() {
    let mut env = setup_test_env();

    // Test staking
    let stake_msg = ExecuteMsg::Rewards(
        StakeMsg {
            callback: None,
            withdraw_rewards: false,
        }
        .into(),
    );
    let res = env.app.execute_contract(
        env.user1.clone(),
        env.rewards_addr.clone(),
        &stake_msg,
        &[coin(100, "TOKEN")],
    );
    assert!(res.is_ok(), "Staking should succeed");

    // Query stake info
    let stake_info: StakeInfoResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::StakeInfo {
                staker: env.user1.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        stake_info.amount,
        Uint128::new(100),
        "Stake amount should be 100"
    );

    // Test staking more
    let res = env.app.execute_contract(
        env.user1.clone(),
        env.rewards_addr.clone(),
        &stake_msg,
        &[coin(50, "TOKEN")],
    );
    assert!(res.is_ok(), "Additional staking should succeed");

    // Query updated stake info
    let stake_info: StakeInfoResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::StakeInfo {
                staker: env.user1.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        stake_info.amount,
        Uint128::new(150),
        "Total stake amount should be 150"
    );
}

#[test]
fn test_unstake_native_token() {
    let mut env = setup_test_env();

    // Stake some tokens first
    env.app
        .execute_contract(
            env.user1.clone(),
            env.rewards_addr.clone(),
            &ExecuteMsg::Rewards(
                StakeMsg {
                    callback: None,
                    withdraw_rewards: false,
                }
                .into(),
            ),
            &[coin(100, "TOKEN")],
        )
        .unwrap();

    // Test unstaking
    let unstake_msg = ExecuteMsg::Rewards(
        UnstakeMsg {
            amount: Uint128::new(50),
            callback: None,
            withdraw_rewards: false,
        }
        .into(),
    );
    let res = env.app.execute_contract(
        env.user1.clone(),
        env.rewards_addr.clone(),
        &unstake_msg,
        &[],
    );
    assert!(res.is_ok(), "Unstaking should succeed");

    // Query stake info after unstaking
    let stake_info: StakeInfoResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::StakeInfo {
                staker: env.user1.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        stake_info.amount,
        Uint128::new(50),
        "Remaining stake amount should be 50"
    );

    // Test unstaking more than staked
    let unstake_msg = ExecuteMsg::Rewards(
        UnstakeMsg {
            amount: Uint128::new(100),
            callback: None,
            withdraw_rewards: false,
        }
        .into(),
    );
    let res = env.app.execute_contract(
        env.user1.clone(),
        env.rewards_addr.clone(),
        &unstake_msg,
        &[],
    );
    assert!(res.is_err(), "Unstaking more than staked should fail");
}

#[test]
fn test_query_weights() {
    let mut env = setup_test_env();

    // Stake some tokens
    env.app
        .execute_contract(
            env.user1.clone(),
            env.rewards_addr.clone(),
            &ExecuteMsg::Rewards(
                StakeMsg {
                    callback: None,
                    withdraw_rewards: false,
                }
                .into(),
            ),
            &[coin(100, "TOKEN")],
        )
        .unwrap();

    env.app
        .execute_contract(
            env.user2.clone(),
            env.rewards_addr.clone(),
            &ExecuteMsg::Rewards(
                StakeMsg {
                    callback: None,
                    withdraw_rewards: false,
                }
                .into(),
            ),
            &[coin(150, "TOKEN")],
        )
        .unwrap();

    // Query weights
    let weights: Vec<StakeInfoResponse> = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::Weights {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();

    assert_eq!(weights.len(), 2, "Should have 2 stakers");
    assert_eq!(
        weights[0].amount,
        Uint128::new(100),
        "User1 should have 100 staked"
    );
    assert_eq!(
        weights[1].amount,
        Uint128::new(150),
        "User2 should have 150 staked"
    );
}
