use cosmwasm_std::{coin, coins, Decimal, Uint128};
use cw_multi_test::Executor;
use rewards_interfaces::modules::{DistributionConfig, Whitelist};
use rewards_interfaces::{msg::*, *};

use super::test_helpers::setup_test_env;

#[test]
fn test_distribute_rewards() {
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
            &[coin(50, "TOKEN")],
        )
        .unwrap();

    // Distribute rewards
    let distribute_msg = ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into());
    let res = env.app.execute_contract(
        env.admin.clone(),
        env.rewards_addr.clone(),
        &distribute_msg,
        &coins(300, "TOKEN"),
    );
    assert!(res.is_ok(), "Distributing rewards should succeed");

    // Query pending rewards for user1
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
        vec![coin(200, "TOKEN")],
        "User1 should have 200 TOKEN in pending rewards"
    );

    // Query pending rewards for user2
    let pending_rewards: PendingRewardsResponse = env
        .app
        .wrap()
        .query_wasm_smart(
            &env.rewards_addr,
            &QueryMsg::PendingRewards {
                staker: env.user2.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        pending_rewards.rewards,
        vec![coin(100, "TOKEN")],
        "User2 should have 100 TOKEN in pending rewards"
    );
}

#[test]
fn test_claim_rewards() {
    let mut env = setup_test_env();

    // Stake some tokens and distribute rewards
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
            env.admin.clone(),
            env.rewards_addr.clone(),
            &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
            &coins(300, "TOKEN"),
        )
        .unwrap();

    // Claim rewards
    let claim_msg = ExecuteMsg::Rewards(ClaimRewardsMsg { callback: None }.into());
    let res =
        env.app
            .execute_contract(env.user1.clone(), env.rewards_addr.clone(), &claim_msg, &[]);
    assert!(res.is_ok(), "Claiming rewards should succeed");

    // Check user1's balance after claiming
    let balance = env.app.wrap().query_balance(&env.user1, "TOKEN").unwrap();
    assert_eq!(
        balance.amount,
        Uint128::new(1_000_200),
        "User1 should have 1,000,200 TOKEN after claiming rewards"
    );

    // Check that pending rewards are now zero
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
    assert!(
        pending_rewards.rewards.is_empty(),
        "Pending rewards should be empty after claiming"
    );
}

#[test]
fn test_distribution_with_fees() {
    let mut env = setup_test_env();

    // Set up fees
    let update_config_msg = ExecuteMsg::UpdateConfig(ConfigUpdate {
        owner: None,
        staking_cfg: None,
        incentive_cfg: None,
        distribution_cfg: Some(ModuleUpdate {
            update: Some(DistributionConfig {
                fees: vec![
                    (Decimal::percent(5), env.admin.clone()),
                    (Decimal::percent(5), env.admin.clone()),
                ],
                whitelisted_denoms: Whitelist::All,
            }),
        }),
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

    // Get admin's balance before distribution
    let admin_balance_before = env.app.wrap().query_balance(&env.admin, "TOKEN").unwrap();

    // Distribute rewards
    env.app
        .execute_contract(
            env.admin.clone(),
            env.rewards_addr.clone(),
            &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
            &coins(1000, "TOKEN"),
        )
        .unwrap();

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
        vec![coin(900, "TOKEN")],
        "User1 should have 900 TOKEN in pending rewards after fees"
    );

    // Check admin's balance (should have received fees)
    let admin_balance_after = env.app.wrap().query_balance(&env.admin, "TOKEN").unwrap();
    let fee_received =
        admin_balance_after.amount - (admin_balance_before.amount - Uint128::from(1000u128));
    assert_eq!(
        fee_received,
        Uint128::new(100),
        "Admin should have received 100 TOKEN in fees"
    );
}

#[test]
fn test_distribution_with_whitelisted_denoms() {
    let mut env = setup_test_env();

    // Set up whitelist
    let update_config_msg = ExecuteMsg::UpdateConfig(ConfigUpdate {
        owner: None,
        staking_cfg: None,
        incentive_cfg: None,
        distribution_cfg: Some(ModuleUpdate {
            update: Some(DistributionConfig {
                fees: vec![],
                whitelisted_denoms: Whitelist::Some(vec![
                    "TOKEN".to_string(),
                    "REWARD".to_string(),
                ]),
            }),
        }),
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

    // Distribute whitelisted rewards
    let res = env.app.execute_contract(
        env.admin.clone(),
        env.rewards_addr.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &[coin(100, "TOKEN"), coin(50, "REWARD")],
    );
    assert!(
        res.is_ok(),
        "Distributing whitelisted rewards should succeed"
    );

    // Try to distribute non-whitelisted rewards
    let res = env.app.execute_contract(
        env.admin.clone(),
        env.rewards_addr.clone(),
        &ExecuteMsg::Rewards(DistributeRewardsMsg { callback: None }.into()),
        &[coin(100, "NONWHITELISTED")],
    );
    assert!(
        res.is_err(),
        "Distributing non-whitelisted rewards should fail"
    );
}
