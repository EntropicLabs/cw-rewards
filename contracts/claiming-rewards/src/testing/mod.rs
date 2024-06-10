#![cfg(test)]
mod multi;

use cosmwasm_std::{coin, coins, Decimal, StdResult, Uint128};
use cw_multi_test::Executor;
use kujira::fee_address;
use rewards_interfaces::{claiming::*, *};
use rewards_tests::CustomApp;

use crate::testing::multi::{add_default_incentive, setup_env};

use self::multi::Addrs;

fn set_default_weights(app: &mut CustomApp, a: &Addrs) {
    app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
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
    app.execute_contract(
        a.user2.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            StakeMsg {
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[coin(50, "TOKEN")],
    )
    .unwrap();
}

#[test]
fn test_stake() {
    let (mut app, a) = setup_env();
    add_default_incentive(&mut app, &a);

    app.update_block(|b| {
        b.height += 1;
        b.time = b.time.plus_seconds(30);
    });

    // Check that pending rewards with zero staked still works.
    let res = app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            StakeMsg {
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[coin(100, "TOKEN")],
    );
    assert!(res.is_ok(), "{res:?}");

    // Check the staked amount
    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::from(100u128));

    // Now, pending rewards should be non-zero for user1
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(pending.rewards, vec![coin(375, "OTHER_TOKEN")]);

    // Stake other user
    let res = app.execute_contract(
        a.user2.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            StakeMsg {
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[coin(50, "TOKEN")],
    );
    assert!(res.is_ok(), "{res:?}");

    // Pending rewards still 375 for user 1, 0 for user 2
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    assert_eq!(pending.unwrap().rewards, vec![coin(375, "OTHER_TOKEN")]);
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user2.clone(),
        },
    );
    assert!(pending.is_ok());
    assert!(pending.unwrap().rewards.is_empty());

    // Check the staked amount
    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user2.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::from(50u128));

    // Stake again with the same user
    let res = app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            StakeMsg {
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[coin(100, "TOKEN")],
    );
    assert!(res.is_ok());

    // Check the staked amount
    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::from(200u128));

    // Stake w/ 0
    let res = app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            StakeMsg {
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[],
    );
    assert!(res.is_err());

    // Advance time to end of the incentive
    app.update_block(|b| {
        b.height += 1;
        b.time = b.time.plus_seconds(30);
    });

    // Pending should be 375 + (4/5 * 375) for user 1, 1/5 * 375 for user 2
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    assert_eq!(
        pending.unwrap().rewards,
        vec![coin(375 + 300, "OTHER_TOKEN")]
    );
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user2.clone(),
        },
    );
    assert!(pending.is_ok());
    assert_eq!(pending.unwrap().rewards, vec![coin(75, "OTHER_TOKEN")]);
}

#[test]
fn test_unstake() {
    let (mut app, a) = setup_env();
    set_default_weights(&mut app, &a);

    let res = app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            UnstakeMsg {
                amount: Uint128::from(100u128),
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[],
    );
    assert!(res.is_ok());

    // Check the staked amount
    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::from(50u128));

    // Unstake all
    let res = app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            UnstakeMsg {
                amount: Uint128::from(50u128),
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[],
    );
    assert!(res.is_ok());

    // Check the staked amount
    let stake_info: StdResult<StakeInfoResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::StakeInfo {
            staker: a.user.clone(),
        },
    );
    assert!(stake_info.is_ok());
    let stake_info = stake_info.unwrap();
    assert_eq!(stake_info.amount, Uint128::zero());

    // Try unstake more than staked
    let res = app.execute_contract(
        a.user2.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            UnstakeMsg {
                amount: Uint128::from(100u128),
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[],
    );
    assert!(res.is_err());
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
        Uint128::from(750u128 + 1_000_000_000_000u128 - 150u128)
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
        stake_denom: None,
        fees: Some(vec![
            (Decimal::percent(10), fee_address()),
            (Decimal::percent(10), fee_address()),
        ]),
        whitelisted_rewards: None,
        underlying_rewards: None,
    };
    set_default_weights(&mut app, &a);
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::UpdateConfig(config_update),
        &[],
    )
    .unwrap();

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

#[test]
fn test_claim_after_unstake_all() {
    let (mut app, a) = setup_env();
    set_default_weights(&mut app, &a);

    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    )
    .unwrap();

    app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            UnstakeMsg {
                amount: Uint128::from(150u128),
                callback: None,
                withdraw_rewards: false,
            }
            .into(),
        ),
        &[],
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

    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert!(pending.rewards.is_empty());
}

#[test]
fn test_inline_withdraw_rewards() {
    let (mut app, a) = setup_env();
    set_default_weights(&mut app, &a);

    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    )
    .unwrap();

    // first stake
    let res = app.execute_contract(
        a.user.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(
            StakeMsg {
                callback: None,
                withdraw_rewards: true,
            }
            .into(),
        ),
        &[coin(150, "TOKEN")],
    );
    assert!(res.is_ok());
    //check balance
    let user_balance = app.wrap().query_balance(&a.user, "TOKEN").unwrap();
    assert_eq!(
        user_balance.amount,
        Uint128::from(750u128 + 1_000_000_000_000u128 - 300u128)
    );

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
        &ExecuteMsg::Rewards(
            UnstakeMsg {
                amount: Uint128::from(300u128),
                callback: None,
                withdraw_rewards: true,
            }
            .into(),
        ),
        &[],
    );
    assert!(res.is_ok());

    let user_balance = app.wrap().query_balance(&a.user, "TOKEN").unwrap();
    assert_eq!(
        user_balance.amount,
        Uint128::from(750u128 + 1_000_000_000_000u128 + /* 300/350 * 1000 */ 857u128)
    );

    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert!(pending.rewards.is_empty());
}
