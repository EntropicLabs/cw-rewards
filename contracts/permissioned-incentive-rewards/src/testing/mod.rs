#![cfg(test)]
mod multi;

use cosmwasm_std::{coin, coins, Decimal, StdResult, Uint128};
use cw_multi_test::Executor;
use kujira::{bow::staking::IncentivesResponse, fee_address, Schedule};
use rewards_interfaces::{permissioned_incentive::*, *};
use rewards_tests::CustomApp;

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

#[test]
fn test_incentives() {
    let (mut app, a) = setup_env();
    set_default_weights(&mut app, &a);

    // add incentive
    let now = app.block_info().time;
    let end = now.plus_seconds(60);
    let res = app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::AddIncentive {
            denom: "OTHER_TOKEN".into(),
            schedule: Schedule {
                start: now,
                end,
                amount: 1_000u128.into(),
                release: kujira::Release::Fixed,
            },
        },
        &[coin(1_000, "OTHER_TOKEN"), coin(100, "TOKEN")],
    );

    assert!(res.is_ok(), "{res:?}");

    // Check incentive query
    let incentives: StdResult<IncentivesResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::Incentives {
            start_after: None,
            limit: None,
        },
    );
    assert!(incentives.is_ok());
    let incentives = incentives.unwrap();
    assert_eq!(incentives.incentives.len(), 1);

    // Check pending right now
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert!(pending.rewards.is_empty());

    // Advance time by 30 seconds
    app.update_block(|b| {
        b.height += 1;
        b.time = b.time.plus_seconds(30);
    });

    // Check pending after 30 seconds
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(pending.rewards, vec![coin(375, "OTHER_TOKEN")]);

    // Do some bogus action to crank the contract
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    )
    .unwrap();

    // Check pending again, should be same, but with bogus action rewards
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(
        pending.rewards,
        vec![coin(375, "OTHER_TOKEN"), coin(750, "TOKEN")]
    );

    // Check incentive query
    let incentives: StdResult<IncentivesResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::Incentives {
            start_after: None,
            limit: None,
        },
    );
    assert!(incentives.is_ok());
    let incentives = incentives.unwrap();
    assert_eq!(incentives.incentives.len(), 1);

    // Advance time by 30 seconds
    app.update_block(|b| {
        b.height += 1;
        b.time = b.time.plus_seconds(30);
    });

    // Check pending after 60 seconds
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(
        pending.rewards,
        vec![coin(750, "OTHER_TOKEN"), coin(750, "TOKEN")]
    );

    // Advance time past incentive end
    app.update_block(|b| {
        b.height += 1;
        b.time = b.time.plus_seconds(30);
    });

    // Check pending after 90 seconds
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(
        pending.rewards,
        vec![coin(750, "OTHER_TOKEN"), coin(750, "TOKEN")]
    );

    // Crank contract again
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    )
    .unwrap();

    // Check pending after 90 seconds
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(
        pending.rewards,
        vec![coin(750, "OTHER_TOKEN"), coin(1500, "TOKEN")]
    );

    // Check incentive query
    let incentives: StdResult<IncentivesResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::Incentives {
            start_after: None,
            limit: None,
        },
    );
    assert!(incentives.is_ok());
    let incentives = incentives.unwrap();
    assert!(incentives.incentives.is_empty());
}

#[test]
fn test_many_many_incentives() {
    let (mut app, a) = setup_env();
    set_default_weights(&mut app, &a);

    // add incentive
    let now = app.block_info().time;
    let end = now.plus_seconds(60);
    for _ in 0..100 {
        let res = app.execute_contract(
            a.admin.clone(),
            a.rewards.clone(),
            &ExecuteMsg::AddIncentive {
                denom: "OTHER_TOKEN".into(),
                schedule: Schedule {
                    start: now,
                    end,
                    amount: 1_000u128.into(),
                    release: kujira::Release::Fixed,
                },
            },
            &[coin(1_000, "OTHER_TOKEN"), coin(100, "TOKEN")],
        );

        assert!(res.is_ok(), "{res:?}");
    }

    // Check incentive query, should be capped to 30
    let incentives: StdResult<IncentivesResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::Incentives {
            start_after: None,
            limit: None,
        },
    );
    assert!(incentives.is_ok());
    let incentives = incentives.unwrap();
    assert_eq!(incentives.incentives.len(), 30);

    // Check incentive query with limit set
    let incentives: StdResult<IncentivesResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::Incentives {
            start_after: None,
            limit: Some(1000),
        },
    );
    assert!(incentives.is_ok());
    let incentives = incentives.unwrap();
    assert_eq!(incentives.incentives.len(), 100);

    // Advance time by 30 seconds
    app.update_block(|b| {
        b.height += 1;
        b.time = b.time.plus_seconds(30);
    });

    // Check pendings, should be using the config.incentive_crank_limit
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(pending.rewards, vec![coin(375 * 10, "OTHER_TOKEN")]);

    // Crank
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    )
    .unwrap();

    // Check pendings, should be using the config.incentive_crank_limit
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(
        pending.rewards,
        vec![coin(375 * 20, "OTHER_TOKEN"), coin(750, "TOKEN")]
    );

    // Crank
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    )
    .unwrap();

    // Check pendings, should be using the config.incentive_crank_limit
    let pending: StdResult<PendingRewardsResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::PendingRewards {
            staker: a.user.clone(),
        },
    );
    assert!(pending.is_ok());
    let pending = pending.unwrap();
    assert_eq!(
        pending.rewards,
        vec![coin(375 * 30, "OTHER_TOKEN"), coin(1500, "TOKEN")]
    );

    // Check incentive query with limit set
    let incentives: StdResult<IncentivesResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::Incentives {
            start_after: None,
            limit: Some(1000),
        },
    );
    assert!(incentives.is_ok());
    let incentives = incentives.unwrap();
    assert_eq!(incentives.incentives.len(), 100);

    // Advance time past incentive end
    app.update_block(|b| {
        b.height += 1;
        b.time = b.time.plus_seconds(31);
    });

    // Crank
    app.execute_contract(
        a.admin.clone(),
        a.rewards.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &coins(1_000, "TOKEN"),
    )
    .unwrap();

    // Check incentive query with limit set
    let incentives: StdResult<IncentivesResponse> = app.wrap().query_wasm_smart(
        &a.rewards,
        &QueryMsg::Incentives {
            start_after: None,
            limit: Some(1000),
        },
    );
    assert!(incentives.is_ok());
    let incentives = incentives.unwrap();
    assert_eq!(incentives.incentives.len(), 90);
}
