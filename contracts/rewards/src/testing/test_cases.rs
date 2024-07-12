use cosmwasm_std::{coin, coins, Decimal, Uint128};
use kujira::{bow::staking::IncentivesResponse, Denom, Release, Schedule};

use crate::{msg::*, Config};
use cw_rewards_logic::*;

use super::{
    test_helpers::TestEnv,
    test_macros::{create_config, define_test},
};

define_test! {
    name: test_native_token_staking,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(300, "utoken")).unwrap();

        env.assert_stake("alice", 500);
        env.assert_stake("bob", 300);

        env.unstake("alice", 200).unwrap();
        env.assert_stake("alice", 300);
        env.assert_balance("alice", coin(700, "utoken"));

        // Attempt to unstake more than staked
        env.unstake("bob", 400).unwrap_err();

        // Attempt to manually adjust weights
        env.adjust_weights("owner", vec![
            ("alice", Uint128::new(100)),
        ]).unwrap_err();
    }
}

define_test! {
    name: test_distribution_with_fees,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![(Decimal::percent(10), multi_app().api().addr_make("fee_collector"))],
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: coins(2000, "utoken"),
        fee_collector: coins(0, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();
        env.distribute_rewards("carol", coins(1000, "utoken")).unwrap();

        env.assert_pending_rewards("alice", vec![coin(450, "utoken")]);
        env.assert_pending_rewards("bob", vec![coin(450, "utoken")]);
        env.assert_balance("fee_collector", coin(100, "utoken"));

        env.claim_rewards("alice").unwrap();
        env.assert_balance("alice", coin(950, "utoken"));
        env.assert_pending_rewards("alice", vec![]);

        // Attempt to distribute zero rewards
        env.distribute_rewards("carol", vec![]).unwrap_err();
    }
}

define_test! {
    name: test_whitelisted_rewards,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::Some(vec!["utoken".to_string(), "ureward".to_string()]),
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: vec![coin(1000, "utoken"), coin(1000, "ureward"), coin(1000, "unotwhitelisted")],
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();

        env.distribute_rewards("carol", vec![coin(1000, "utoken"), coin(1000, "ureward")]).unwrap();
        env.assert_pending_rewards("alice", vec![coin(500, "utoken"), coin(500, "ureward")]);

        env.distribute_rewards("carol", coins(1000, "unotwhitelisted")).unwrap_err();
    }
}

define_test! {
    name: test_incentives,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
        incentive: {
            crank_limit: 10,
            min_size: Uint128::new(100),
            fee: Some(coin(10, "utoken")),
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: vec![coin(2000, "utoken"), coin(2000, "ureward")],
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();

        let now = env.block_time();
        let end = now.plus_seconds(3600);
        env.add_incentive("carol", "ureward", Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "ureward"), coin(10, "utoken")]).unwrap();

        env.advance_time(1800);  // Advance halfway through the incentive period
        env.assert_pending_rewards("alice", vec![coin(250, "ureward")]);
        env.assert_pending_rewards("bob", vec![coin(250, "ureward")]);

        env.advance_time(1800);  // Advance to the end of the incentive period
        env.assert_pending_rewards("alice", vec![coin(500, "ureward")]);
        env.assert_pending_rewards("bob", vec![coin(500, "ureward")]);

        // Attempt to add an incentive below min_size
        env.add_incentive("carol", "ureward", Schedule {
            start: now,
            end,
            amount: Uint128::new(50),
            release: Release::Fixed,
        }, vec![coin(50, "ureward"), coin(10, "utoken")]).unwrap_err();
    }
}

define_test! {
    name: test_update_config,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1500, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        env.update_config("owner", ConfigUpdate {
            owner: Some(env.addr("new_owner")),
            distribution_cfg: Some(ModuleUpdate {
                update: Some(DistributionConfig {
                    fees: vec![(Decimal::percent(5), env.addr("fee_collector"))],
                    whitelisted_denoms: Whitelist::Some(vec!["utoken".to_string()]),
                }),
            }),
            incentive_cfg: None,
            staking_cfg: None,
            underlying_cfg: None,
        }).unwrap();

        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();
        env.distribute_rewards("bob", coins(1000, "utoken")).unwrap();

        env.assert_pending_rewards("alice", vec![coin(475, "utoken")]);
        env.assert_pending_rewards("bob", vec![coin(475, "utoken")]);
        env.assert_balance("fee_collector", coin(50, "utoken"));
    }
}

define_test! {
    name: test_incentive_without_module,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
        // Note: No incentive module configured
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        let now = env.block_time();
        let end = now.plus_seconds(3600);
        env.add_incentive("alice", "ureward", Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "ureward"), coin(10, "utoken")]).unwrap_err();
    }
}

define_test! {
    name: test_incentive_starting_in_past,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
        incentive: {
            crank_limit: 10,
            min_size: Uint128::new(100),
            fee: Some(coin(10, "utoken")),
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: vec![coin(2000, "utoken"), coin(2000, "ureward")],
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();

        let now = env.block_time();
        let start = now.minus_seconds(1800); // 30 minutes in the past
        let end = now.plus_seconds(1800);    // 30 minutes in the future
        env.add_incentive("carol", "ureward", Schedule {
            start,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "ureward"), coin(10, "utoken")]).unwrap();

        // Check that half the rewards were instantly distributed
        env.assert_pending_rewards("alice", vec![coin(250, "ureward")]);
        env.assert_pending_rewards("bob", vec![coin(250, "ureward")]);
    }
}

define_test! {
    name: test_permissioned_weight_adjustment,
    config: {
        owner: "owner",
        staking: Permissioned(""),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: coins(2000, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        env.adjust_weights("owner", vec![
            ("alice", Uint128::new(500)),
            ("bob", Uint128::new(300)),
        ]).unwrap();

        env.assert_stake("alice", 500);
        env.assert_stake("bob", 300);

        env.distribute_rewards("carol", coins(800, "utoken")).unwrap();

        env.assert_pending_rewards("alice", vec![coin(500, "utoken")]);
        env.assert_pending_rewards("bob", vec![coin(300, "utoken")]);

        // Non-owner cannot adjust weights
        env.adjust_weights("alice", vec![
            ("alice", Uint128::new(1000)),
        ]).unwrap_err();

        // Staking should fail with Permissioned config
        env.stake("alice", coin(100, "utoken")).unwrap_err();
    }
}

define_test! {
    name: test_invalid_staking_config,
    config: {
        owner: "owner",
        staking: Permissioned(""),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        // Attempting to stake with NativeToken method should fail
        env.stake("alice", coin(500, "utoken")).unwrap_err();

        // Attempting to unstake with NativeToken method should fail
        env.unstake("bob", 300).unwrap_err();
    }
}

define_test! {
    name: test_distribution_without_module,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        // Note: No distribution module configured
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: coins(2000, "utoken"),
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();

        // Attempting to distribute rewards should fail
        env.distribute_rewards("carol", coins(1000, "utoken")).unwrap_err();
    }
}

define_test! {
    name: test_distribution_whitelist,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::Some(vec!["utoken".to_string(), "ureward".to_string()]),
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: vec![coin(1000, "utoken"), coin(1000, "ureward"), coin(1000, "unotwhitelisted")],
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();

        // Distribute whitelisted rewards
        env.distribute_rewards("carol", vec![coin(1000, "utoken"), coin(1000, "ureward")]).unwrap();
        env.assert_pending_rewards("alice", vec![coin(500, "utoken"), coin(500, "ureward")]);
        env.assert_pending_rewards("bob", vec![coin(500, "utoken"), coin(500, "ureward")]);

        // Attempt to distribute non-whitelisted rewards
        env.distribute_rewards("carol", coins(1000, "unotwhitelisted")).unwrap_err();
    }
}

define_test! {
    name: test_incentive_whitelist,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
        incentive: {
            crank_limit: 10,
            min_size: Uint128::new(100),
            fee: Some(coin(10, "utoken")),
            whitelisted_denoms: Whitelist::Some(vec!["ureward".to_string(), "uincentive".to_string()]),
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: vec![coin(2000, "utoken"), coin(2000, "ureward"), coin(2000, "uincentive"), coin(2000, "unotwhitelisted")],
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();

        let now = env.block_time();
        let end = now.plus_seconds(3600);

        // Add whitelisted incentive
        env.add_incentive("carol", "ureward", Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "ureward"), coin(10, "utoken")]).unwrap();

        // Attempt to add non-whitelisted incentive
        env.add_incentive("carol", "unotwhitelisted", Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "unotwhitelisted"), coin(10, "utoken")]).unwrap_err();
    }
}

define_test! {
    name: test_different_whitelists,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::Some(vec!["utoken".to_string(), "ureward".to_string()]),
        },
        incentive: {
            crank_limit: 10,
            min_size: Uint128::new(100),
            fee: Some(coin(10, "utoken")),
            whitelisted_denoms: Whitelist::Some(vec!["uincentive".to_string(), "ureward".to_string()]),
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: vec![coin(2000, "utoken"), coin(2000, "ureward"), coin(2000, "uincentive")],
    },
    test_fn: |env: &mut TestEnv| {
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(500, "utoken")).unwrap();

        let now = env.block_time();
        let end = now.plus_seconds(3600);

        // Distribute rewards using distribution whitelist
        env.distribute_rewards("carol", vec![coin(1000, "utoken"), coin(1000, "ureward")]).unwrap();
        env.assert_pending_rewards("alice", vec![coin(500, "utoken"), coin(500, "ureward")]);

        // Add incentive using incentive whitelist
        env.add_incentive("carol", "uincentive", Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "uincentive"), coin(10, "utoken")]).unwrap();

        // Attempt to distribute rewards using incentive whitelist (should fail)
        env.distribute_rewards("carol", coins(1000, "uincentive")).unwrap_err();

        // Attempt to add incentive using distribution whitelist (should fail)
        env.add_incentive("carol", "utoken", Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "utoken"), coin(10, "utoken")]).unwrap_err();

        // Add incentive using shared whitelist denom
        env.add_incentive("carol", "ureward", Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "ureward"), coin(10, "utoken")]).unwrap();
    }
}

define_test! {
    name: test_underlying_rewards,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
        // We'll set the underlying_rewards_module after instantiating the second contract
    },
    accounts: {
        alice: vec![coin(1000, "utoken"), coin(1000, "ureward")],
        bob: vec![coin(1000, "utoken"), coin(1000, "ureward")],
        carol: vec![coin(2000, "utoken"), coin(2000, "ureward")],
    },
    test_fn: |env: &mut TestEnv| do_test_underlying_rewards(env),
}

fn do_test_underlying_rewards(env: &mut TestEnv) {
    // Instantiate the second (underlying) rewards contract
    let underlying_msg = create_config! {
        app: &env.app,
        owner: "owner",
        staking: Permissioned(""),
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
    };
    let underlying = env
        .instantiate(underlying_msg, env.rewards_code_id, "underlying rewards")
        .unwrap();

    // Update the main contract to use the underlying rewards contract
    env.update_config(
        "owner",
        ConfigUpdate {
            underlying_cfg: Some(ModuleUpdate {
                update: Some(UnderlyingConfig {
                    underlying_rewards_contract: underlying.clone(),
                }),
            }),
            ..Default::default()
        },
    )
    .unwrap();

    // Stake in the main contract
    env.stake("alice", coin(500, "utoken")).unwrap();
    env.stake("bob", coin(500, "utoken")).unwrap();

    // Set weights in the underlying contract
    env.execute(
        "owner",
        &underlying,
        &ExecuteMsg::AdjustWeights {
            delta: vec![(env.rewards_addr.clone(), 1000u128.into())],
        },
        vec![],
    )
    .unwrap();

    // Distribute rewards to the underlying contract
    env.execute(
        "carol",
        &underlying,
        &ExecuteMsg::Rewards(RewardsMsg::DistributeRewards(DistributeRewardsMsg {
            callback: None,
        })),
        vec![coin(1000, "ureward")],
    )
    .unwrap();

    // Trigger distribution in the main contract
    env.distribute_rewards("carol", coins(100, "utoken"))
        .unwrap();

    // Check pending rewards in the main contract
    env.assert_pending_rewards("alice", vec![coin(50, "utoken"), coin(500, "ureward")]);
    env.assert_pending_rewards("bob", vec![coin(50, "utoken"), coin(500, "ureward")]);

    // Claim rewards from the main contract
    env.claim_rewards("alice").unwrap();

    // Check balances after claiming
    env.assert_balance("alice", coin(550, "utoken"));
    env.assert_balance("alice", coin(1500, "ureward"));

    // Unstake from the main contract
    env.unstake("alice", 250).unwrap();

    // Distribute more rewards to the underlying contract
    env.execute(
        "carol",
        &underlying,
        &ExecuteMsg::Rewards(RewardsMsg::DistributeRewards(DistributeRewardsMsg {
            callback: None,
        })),
        vec![coin(1000, "ureward")],
    )
    .unwrap();

    // Trigger distribution in the main contract again
    env.distribute_rewards("carol", coins(100, "utoken"))
        .unwrap();

    // Check pending rewards, ensuring they're proportional to the new stakes
    env.assert_pending_rewards("alice", vec![coin(33, "utoken"), coin(333, "ureward")]);
    env.assert_pending_rewards(
        "bob",
        vec![coin(66 + 50, "utoken"), coin(666 + 500, "ureward")],
    );
}

define_test! {
    name: test_cw4_hook,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"), // Will be updated to Cw4Hook after instantiating the cw4
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: vec![coin(1000, "utoken"), coin(1000, "ureward")],
        bob: vec![coin(1000, "utoken"), coin(1000, "ureward")],
        carol: vec![coin(2000, "utoken"), coin(2000, "ureward")],
    },
    test_fn: |env: &mut TestEnv| do_test_cw4_hook(env),
}

fn do_test_cw4_hook(env: &mut TestEnv) {
    // Instantiate the cw4 contract
    let cw4 = env
        .instantiate(
            cw4_stake::msg::InstantiateMsg {
                denom: cw20::Denom::Native("utoken".to_string()),
                tokens_per_weight: 1u128.into(),
                min_bond: Uint128::zero(),
                unbonding_period: cw_utils::Duration::Time(60),
                admin: Some(env.owner.to_string()),
            },
            env.cw4_code_id,
            "cw4",
        )
        .unwrap();

    // Add the rewards to the cw4 hooks
    env.execute(
        "owner",
        &cw4,
        &cw4_stake::msg::ExecuteMsg::AddHook {
            addr: env.rewards_addr.to_string(),
        },
        vec![],
    )
    .unwrap();

    // Bonding tokens with the wrong StakingConfig should fail
    env.execute(
        "alice",
        &cw4,
        &cw4_stake::msg::ExecuteMsg::Bond {},
        vec![coin(500, "utoken")],
    )
    .unwrap_err();

    // Update the main contract to use the WRONG cw4 address
    env.update_config(
        "owner",
        ConfigUpdate {
            staking_cfg: Some(ModuleUpdate {
                update: StakingConfig::Cw4Hook {
                    cw4_addr: env.addr("wrong_cw4"),
                },
            }),
            ..Default::default()
        },
    )
    .unwrap();

    // Receiving unauthorized hooks should fail
    env.execute(
        "alice",
        &cw4,
        &cw4_stake::msg::ExecuteMsg::Bond {},
        vec![coin(500, "utoken")],
    )
    .unwrap_err();

    // Update the main contract to use the cw4 address
    env.update_config(
        "owner",
        ConfigUpdate {
            staking_cfg: Some(ModuleUpdate {
                update: StakingConfig::Cw4Hook {
                    cw4_addr: cw4.clone(),
                },
            }),
            ..Default::default()
        },
    )
    .unwrap();

    // Bond tokens for Alice and Bob
    env.execute(
        "alice",
        &cw4,
        &cw4_stake::msg::ExecuteMsg::Bond {},
        vec![coin(500, "utoken")],
    )
    .unwrap();

    env.execute(
        "bob",
        &cw4,
        &cw4_stake::msg::ExecuteMsg::Bond {},
        vec![coin(300, "utoken")],
    )
    .unwrap();

    // Check stakes in the rewards contract
    env.assert_stake("alice", 500);
    env.assert_stake("bob", 300);

    // Distribute rewards
    env.distribute_rewards("carol", vec![coin(800, "ureward")])
        .unwrap();

    // Check pending rewards
    env.assert_pending_rewards("alice", vec![coin(500, "ureward")]);
    env.assert_pending_rewards("bob", vec![coin(300, "ureward")]);

    // Claim rewards
    env.claim_rewards("alice").unwrap();
    env.claim_rewards("bob").unwrap();

    // Check balances after claiming
    env.assert_balance("alice", coin(500, "utoken"));
    env.assert_balance("alice", coin(1500, "ureward"));
    env.assert_balance("bob", coin(700, "utoken"));
    env.assert_balance("bob", coin(1300, "ureward"));

    // Unbond some tokens for Alice
    env.execute(
        "alice",
        &cw4,
        &cw4_stake::msg::ExecuteMsg::Unbond {
            tokens: Uint128::new(200),
        },
        vec![],
    )
    .unwrap();

    // Check updated stakes
    env.assert_stake("alice", 300);
    env.assert_stake("bob", 300);

    // Distribute more rewards
    env.distribute_rewards("carol", vec![coin(600, "ureward")])
        .unwrap();

    // Check pending rewards after unbonding
    env.assert_pending_rewards("alice", vec![coin(300, "ureward")]);
    env.assert_pending_rewards("bob", vec![coin(300, "ureward")]);

    // Advance time to allow claiming unbonded tokens
    env.advance_time(61);

    // Claim unbonded tokens
    env.execute("alice", &cw4, &cw4_stake::msg::ExecuteMsg::Claim {}, vec![])
        .unwrap();

    // Check final balances
    env.assert_balance("alice", coin(700, "utoken"));
    env.assert_balance("alice", coin(1500, "ureward"));
    env.assert_balance("bob", coin(700, "utoken"));
    env.assert_balance("bob", coin(1300, "ureward"));
}

define_test! {
    name: test_daodao_hook,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"), // Will be updated to DaoDaoHook in the test
        distribution: {
            fees: vec![],
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: vec![coin(1000, "utoken"), coin(1000, "ureward")],
        bob: vec![coin(1000, "utoken"), coin(1000, "ureward")],
        carol: vec![coin(2000, "utoken"), coin(2000, "ureward")],
    },
    test_fn: |env: &mut TestEnv| do_test_daodao_hook(env),
}

fn do_test_daodao_hook(env: &mut TestEnv) {
    // Bonding tokens with the wrong StakingConfig should fail
    env.execute(
        "owner",
        &env.rewards_addr.clone(),
        &ExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Stake {
            addr: env.addr("alice").clone(),
            amount: 500u128.into(),
        }),
        vec![],
    )
    .unwrap_err();

    // Update the main contract to use the WRONG DAODAO address
    env.update_config(
        "owner",
        ConfigUpdate {
            staking_cfg: Some(ModuleUpdate {
                update: StakingConfig::DaoDaoHook {
                    daodao_addr: env.addr("wrong_daodao"),
                },
            }),
            ..Default::default()
        },
    )
    .unwrap();

    // Receiving unauthorized hooks should fail
    env.execute(
        "owner",
        &env.rewards_addr.clone(),
        &ExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Stake {
            addr: env.addr("alice"),
            amount: 500u128.into(),
        }),
        vec![],
    )
    .unwrap_err();

    // Update the main contract to use the owner as the hook source.
    env.update_config(
        "owner",
        ConfigUpdate {
            staking_cfg: Some(ModuleUpdate {
                update: StakingConfig::DaoDaoHook {
                    daodao_addr: env.addr("owner"),
                },
            }),
            ..Default::default()
        },
    )
    .unwrap();

    // Bond tokens for Alice and Bob
    env.execute(
        "owner",
        &env.rewards_addr.clone(),
        &ExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Stake {
            addr: env.addr("alice"),
            amount: 500u128.into(),
        }),
        vec![],
    )
    .unwrap();

    env.execute(
        "owner",
        &env.rewards_addr.clone(),
        &ExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Stake {
            addr: env.addr("bob"),
            amount: 300u128.into(),
        }),
        vec![],
    )
    .unwrap();

    // Check stakes in the rewards contract
    env.assert_stake("alice", 500);
    env.assert_stake("bob", 300);

    // Distribute rewards
    env.distribute_rewards("carol", vec![coin(800, "ureward")])
        .unwrap();

    // Check pending rewards
    env.assert_pending_rewards("alice", vec![coin(500, "ureward")]);
    env.assert_pending_rewards("bob", vec![coin(300, "ureward")]);

    // Claim rewards
    env.claim_rewards("alice").unwrap();
    env.claim_rewards("bob").unwrap();

    // Check balances after claiming
    env.assert_balance("alice", coin(1000, "utoken"));
    env.assert_balance("alice", coin(1500, "ureward"));
    env.assert_balance("bob", coin(1000, "utoken"));
    env.assert_balance("bob", coin(1300, "ureward"));

    // Unbond some tokens for Alice
    env.execute(
        "owner",
        &env.rewards_addr.clone(),
        &ExecuteMsg::StakeChangeHook(StakeChangedHookMsg::Unstake {
            addr: env.addr("alice"),
            amount: 200u128.into(),
        }),
        vec![],
    )
    .unwrap();

    // Check updated stakes
    env.assert_stake("alice", 300);
    env.assert_stake("bob", 300);

    // Distribute more rewards
    env.distribute_rewards("carol", vec![coin(600, "ureward")])
        .unwrap();

    // Check pending rewards after unbonding
    env.assert_pending_rewards("alice", vec![coin(300, "ureward")]);
    env.assert_pending_rewards("bob", vec![coin(300, "ureward")]);

    // Check final balances
    env.assert_balance("alice", coin(1000, "utoken"));
    env.assert_balance("alice", coin(1500, "ureward"));
    env.assert_balance("bob", coin(1000, "utoken"));
    env.assert_balance("bob", coin(1300, "ureward"));
}

define_test! {
    name: test_queries,
    config: {
        owner: "owner",
        staking: NativeToken("utoken"),
        distribution: {
            fees: vec![(Decimal::percent(5), multi_app().api().addr_make("fee_collector"))],
            whitelisted_denoms: Whitelist::All,
        },
        incentive: {
            crank_limit: 10,
            min_size: Uint128::new(100),
            fee: Some(coin(10, "utoken")),
            whitelisted_denoms: Whitelist::All,
        },
    },
    accounts: {
        alice: coins(1000, "utoken"),
        bob: coins(1000, "utoken"),
        carol: vec![coin(2000, "utoken"),coin(2000, "ureward")],
    },
    test_fn: |env: &mut TestEnv| {
        // Set up some initial state
        env.stake("alice", coin(500, "utoken")).unwrap();
        env.stake("bob", coin(300, "utoken")).unwrap();
        env.distribute_rewards("carol", coins(800, "utoken")).unwrap();

        let now = env.block_time();
        let end = now.plus_seconds(3600);
        env.add_incentive("carol", "ureward", Schedule {
            start: now,
            end,
            amount: Uint128::new(1000),
            release: Release::Fixed,
        }, vec![coin(1000, "ureward"), coin(10, "utoken")]).unwrap();

        // Test QueryMsg::Config
        let config: Config = env.query(QueryMsg::Config {}).unwrap();
        assert_eq!(config.owner, env.addr("owner"));
        assert_eq!(config.staking_module, StakingConfig::NativeToken{denom: "utoken".to_string()});
        assert!(config.distribution_module.is_some());
        assert!(config.incentive_module.is_some());

        // Test QueryMsg::PendingRewards
        let pending_rewards: PendingRewardsResponse = env.query(QueryMsg::PendingRewards {
            staker: env.addr("alice"),
        }).unwrap();
        assert_eq!(pending_rewards.rewards, vec![coin(475, "utoken")]);

        // Test QueryMsg::StakeInfo
        let stake_info: StakeInfoResponse = env.query(QueryMsg::StakeInfo {
            staker: env.addr("bob"),
        }).unwrap();
        assert_eq!(stake_info.staker, env.addr("bob"));
        assert_eq!(stake_info.amount, Uint128::new(300));

        // Test QueryMsg::Weights
        let weights: Vec<StakeInfoResponse> = env.query(QueryMsg::Weights {
            start_after: None,
            limit: None,
        }).unwrap();
        assert_eq!(weights.len(), 2);
        assert_eq!(weights[0].staker, env.addr("alice"));
        assert_eq!(weights[0].amount, Uint128::new(500));
        assert_eq!(weights[1].staker, env.addr("bob"));
        assert_eq!(weights[1].amount, Uint128::new(300));

        // Test QueryMsg::Weights with pagination
        let weights: Vec<StakeInfoResponse> = env.query(QueryMsg::Weights {
            start_after: Some(env.addr("alice")),
            limit: Some(1),
        }).unwrap();
        assert_eq!(weights.len(), 1);
        assert_eq!(weights[0].staker, env.addr("bob"));
        assert_eq!(weights[0].amount, Uint128::new(300));

        // Test QueryMsg::Incentives
        let incentives: IncentivesResponse = env.query(QueryMsg::Incentives {
            start_after: None,
            limit: None,
        }).unwrap();
        assert_eq!(incentives.incentives.len(), 1);
        assert_eq!(incentives.incentives[0].denom, Denom::from("ureward"));
        assert_eq!(incentives.incentives[0].schedule.amount, Uint128::new(1000));

        // Advance time to test incentive distribution
        env.advance_time(1800);

        // Test QueryMsg::PendingRewards after incentive distribution
        let pending_rewards: PendingRewardsResponse = env.query(QueryMsg::PendingRewards {
            staker: env.addr("alice"),
        }).unwrap();

        assert_eq!(pending_rewards.rewards, vec![coin(312, "ureward"), coin(475, "utoken")]);
    }
}
