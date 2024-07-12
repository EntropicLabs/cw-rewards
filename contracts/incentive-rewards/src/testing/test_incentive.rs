use cosmwasm_std::{coin, Uint128};
use cw_multi_test::Executor;
use kujira::bow::staking::IncentivesResponse;
use kujira::{Release, Schedule};
use rewards_interfaces::modules::{IncentiveConfig, Whitelist};
use rewards_interfaces::{msg::*, *};

use super::test_helpers::setup_test_env;

#[test]
fn test_add_incentive() {
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

    // Add an incentive
    let now = env.app.block_info().time;
    let end = now.plus_seconds(3600); // 1 hour later
    let add_incentive_msg = ExecuteMsg::AddIncentive {
        denom: "REWARD".to_string(),
        schedule: Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        },
    };
    let res = env.app.execute_contract(
        env.admin.clone(),
        env.rewards_addr.clone(),
        &add_incentive_msg,
        &[coin(1000, "REWARD"), coin(100, "TOKEN")], // Sending both the incentive amount and the fee
    );
    assert!(res.is_ok(), "Adding an incentive should succeed");

    // Query incentives
    let incentives: IncentivesResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::Incentives {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(
        incentives.incentives.len(),
        1,
        "There should be one incentive"
    );
    assert_eq!(
        incentives.incentives[0].denom.as_ref(),
        "REWARD",
        "Incentive denom should be REWARD"
    );
    assert_eq!(
        incentives.incentives[0].schedule.amount,
        Uint128::new(1000),
        "Incentive amount should be 1000"
    );
}

#[test]
fn test_incentive_distribution() {
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

    // Add an incentive
    let now = env.app.block_info().time;
    let end = now.plus_seconds(3600); // 1 hour later
    let add_incentive_msg = ExecuteMsg::AddIncentive {
        denom: "REWARD".to_string(),
        schedule: Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        },
    };
    env.app
        .execute_contract(
            env.admin.clone(),
            env.rewards_addr.clone(),
            &add_incentive_msg,
            &[coin(1000, "REWARD"), coin(100, "TOKEN")],
        )
        .unwrap();

    // Advance time by 30 minutes
    env.app.update_block(|b| {
        b.time = b.time.plus_seconds(1800);
    });

    // Query pending rewards
    let pending_rewards: PendingRewardsResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::PendingRewards {
                staker: env.user1.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        pending_rewards.rewards,
        vec![coin(500, "REWARD")],
        "User1 should have 500 REWARD in pending rewards after 30 minutes"
    );

    // Advance time to the end of the incentive period
    env.app.update_block(|b| {
        b.time = end.plus_seconds(1);
    });

    // Query pending rewards again
    let pending_rewards: PendingRewardsResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::PendingRewards {
                staker: env.user1.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        pending_rewards.rewards,
        vec![coin(1000, "REWARD")],
        "User1 should have all 1000 REWARD in pending rewards after incentive period"
    );
}

#[test]
fn test_incentive_crank_limit() {
    let mut env = setup_test_env();

    // Update incentive config with a low crank limit
    let update_config_msg = ExecuteMsg::UpdateConfig(ConfigUpdate {
        owner: None,
        staking_cfg: None,
        incentive_cfg: Some(ModuleUpdate {
            update: Some(IncentiveConfig {
                crank_limit: 2,
                min_size: Uint128::new(100),
                fee: Some(coin(100, "TOKEN")),
                whitelisted_denoms: Whitelist::All,
            }),
        }),
        distribution_cfg: None,
        underlying_cfg: None,
    });
    env.app
        .execute_contract(
            env.admin.clone(),
            env.rewards_addr.clone(),
            &update_config_msg,
            &[],
        )
        .unwrap();

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

    // Add multiple incentives
    let now = env.app.block_info().time;
    let end = now.plus_seconds(3600); // 1 hour later

    // First, add denoms to sender account
    env.app
        .init_modules(|router, _, storage| {
            let balances = vec![
                coin(1000, "REWARD0"),
                coin(1000, "REWARD1"),
                coin(1000, "REWARD2"),
                coin(1000, "REWARD3"),
                coin(1000, "REWARD4"),
                coin(1_000_000, "TOKEN"),
                coin(1_000_000, "REWARD"),
            ];
            router.bank.init_balance(storage, &env.admin, balances)
        })
        .unwrap();

    for i in 0..5 {
        let add_incentive_msg = ExecuteMsg::AddIncentive {
            denom: format!("REWARD{}", i),
            schedule: Schedule {
                start: now,
                end,
                amount: Uint128::new(1000),
                release: Release::Fixed,
            },
        };
        env.app
            .execute_contract(
                env.admin.clone(),
                env.rewards_addr.clone(),
                &add_incentive_msg,
                &[coin(1000, &format!("REWARD{}", i)), coin(100, "TOKEN")],
            )
            .unwrap();
    }

    // Advance time by 30 minutes
    env.app.update_block(|b| {
        b.time = b.time.plus_seconds(1800);
    });

    // Query pending rewards
    let pending_rewards: PendingRewardsResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::PendingRewards {
                staker: env.user1.clone(),
            },
        )
        .unwrap();

    // Check that only rewards from the first two incentives are processed due to crank limit
    assert_eq!(
        pending_rewards.rewards.len(),
        2,
        "Only 2 reward types should be processed due to crank limit"
    );
    assert_eq!(
        pending_rewards.rewards,
        vec![coin(500, "REWARD0"), coin(500, "REWARD1")],
        "User1 should have 500 each of REWARD0 and REWARD1 due to crank limit"
    );

    // Perform another action to trigger incentive processing
    env.app
        .execute_contract(
            env.user1.clone(),
            env.rewards_addr.clone(),
            &ExecuteMsg::Rewards(ClaimRewardsMsg { callback: None }.into()),
            &[],
        )
        .unwrap();

    // Query pending rewards again
    let pending_rewards: PendingRewardsResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::PendingRewards {
                staker: env.user1.clone(),
            },
        )
        .unwrap();

    // Check that rewards from the next two incentives are now processed
    assert_eq!(
        pending_rewards.rewards.len(),
        2,
        "4 reward types should be processed after second action"
    );
    assert_eq!(
        pending_rewards.rewards,
        vec![coin(500, "REWARD2"), coin(500, "REWARD3"),],
        "User1 should have 500 each of REWARD2, and REWARD3 after claiming first two in second action"
    );
}

#[test]
fn test_incentive_min_size_and_fee() {
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

    // Try to add an incentive below min_size
    let now = env.app.block_info().time;
    let end = now.plus_seconds(3600); // 1 hour later
    let add_small_incentive_msg = ExecuteMsg::AddIncentive {
        denom: "SMALL_REWARD".to_string(),
        schedule: Schedule {
            start: now,
            end,
            amount: Uint128::new(50), // Below min_size of 100
            release: Release::Fixed,
        },
    };
    let res = env.app.execute_contract(
        env.admin.clone(),
        env.rewards_addr.clone(),
        &add_small_incentive_msg,
        &[coin(50, "SMALL_REWARD"), coin(100, "TOKEN")],
    );
    assert!(
        res.is_err(),
        "Adding an incentive below min_size should fail"
    );

    // Add a valid incentive
    let add_incentive_msg = ExecuteMsg::AddIncentive {
        denom: "REWARD".to_string(),
        schedule: Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        },
    };
    let res = env.app.execute_contract(
        env.admin.clone(),
        env.rewards_addr.clone(),
        &add_incentive_msg,
        &[coin(1000, "REWARD"), coin(100, "TOKEN")],
    );
    assert!(
        res.is_ok(),
        "Adding a valid incentive should succeed, {res:?}"
    );

    // Check that the fee was taken
    let contract_balance = env
        .app
        .wrap()
        .query_balance(&env.rewards_addr, "TOKEN")
        .unwrap();
    assert_eq!(
        contract_balance.amount,
        Uint128::new(200), // 100 from initial stake + 100 fee
        "Contract should have received the incentive fee"
    );
}

#[test]
fn test_incentive_whitelisted_denoms() {
    let mut env = setup_test_env();

    // Update incentive config with whitelisted denoms
    let update_config_msg = ExecuteMsg::UpdateConfig(ConfigUpdate {
        owner: None,
        staking_cfg: None,
        incentive_cfg: Some(ModuleUpdate {
            update: Some(IncentiveConfig {
                crank_limit: 10,
                min_size: Uint128::new(100),
                fee: Some(coin(100, "TOKEN")),
                whitelisted_denoms: Whitelist::Some(vec!["REWARD".to_string()]),
            }),
        }),
        distribution_cfg: None,
        underlying_cfg: None,
    });
    env.app
        .execute_contract(
            env.admin.clone(),
            env.rewards_addr.clone(),
            &update_config_msg,
            &[],
        )
        .unwrap();

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

    // Add a whitelisted incentive
    let now = env.app.block_info().time;
    let end = now.plus_seconds(3600);
    let add_whitelisted_incentive_msg = ExecuteMsg::AddIncentive {
        denom: "REWARD".to_string(),
        schedule: Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        },
    };
    let res = env.app.execute_contract(
        env.admin.clone(),
        env.rewards_addr.clone(),
        &add_whitelisted_incentive_msg,
        &[coin(1000, "REWARD"), coin(100, "TOKEN")],
    );
    assert!(res.is_ok(), "Adding a whitelisted incentive should succeed");

    // Try to add a non-whitelisted incentive
    let add_non_whitelisted_incentive_msg = ExecuteMsg::AddIncentive {
        denom: "NON_WHITELISTED".to_string(),
        schedule: Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        },
    };
    let res = env.app.execute_contract(
        env.admin.clone(),
        env.rewards_addr.clone(),
        &add_non_whitelisted_incentive_msg,
        &[coin(1000, "NON_WHITELISTED"), coin(100, "TOKEN")],
    );
    assert!(
        res.is_err(),
        "Adding a non-whitelisted incentive should fail"
    );
}
