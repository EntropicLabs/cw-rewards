mod macros {
    macro_rules! create_config {
        (
            app: $app:expr,
            owner: $owner:expr,
            staking: $staking_variant:ident ( $staking_arg:expr ),
            $( distribution: { $( $distribution_key:ident: $distribution_value:expr ),+ $(,)? }, )?
            $( incentive: { $( $incentive_key:ident: $incentive_value:expr ),+ $(,)? }, )?
            $( underlying: $underlying:expr )?
            $(,)?
        ) => {{
            use crate::testing::test_helpers::create_config as create_config_helper;
            #[allow(unused_imports)]
            use rewards_interfaces::modules::{StakingConfig, IncentiveConfig, DistributionConfig, UnderlyingConfig};
            use cosmwasm_std::Addr;

            let staking_module = match stringify!($staking_variant) {
                "NativeToken" => StakingConfig::NativeToken { denom: $staking_arg.to_string() },
                "DaoDaoHook" => StakingConfig::DaoDaoHook { daodao_addr: Addr::unchecked($staking_arg) },
                "Cw4Hook" => StakingConfig::Cw4Hook { cw4_addr: Addr::unchecked($staking_arg) },
                "Permissioned" => StakingConfig::Permissioned {},
                _ => panic!("Invalid staking configuration"),
            };

            let incentive_module = None $(
                .or(Some(IncentiveConfig {
                    $( $incentive_key: $incentive_value, )+
                }))
            )?;

            let distribution_module = None $(
                .or(Some(DistributionConfig {
                    $( $distribution_key: $distribution_value, )+
                }))
            )?;

            let underlying_rewards_module = None $( .or(Some(UnderlyingConfig { underlying_rewards_contract: Addr::unchecked($underlying) })) )?;

            create_config_helper(
                $app,
                $owner,
                staking_module,
                incentive_module,
                distribution_module,
                underlying_rewards_module,
            )
        }};
    }

    macro_rules! define_test {
        (
            name: $name:ident,
            config: {
                $($config:tt)*
            },
            accounts: {
                $($account:ident: $balance:expr),* $(,)?
            },
            test_fn: $test_fn:expr $(,)?
        ) => {
            #[test]
            fn $name() {
                use crate::testing::test_helpers::{setup_test_env, multi_app, TestEnv};

                let app = multi_app();
                // Set up the contract configuration
                let config = create_config! {
                    app: &app,
                    $($config)*
                };

                // Set up the accounts
                let accounts = vec![
                    $(
                        (stringify!($account), $balance),
                    )*
                ];

                // Initialize the test environment
                let mut env = setup_test_env(app, accounts, config);

                // Execute the test function
                $test_fn(&mut env);
            }
        };
    }

    pub(crate) use {create_config, define_test};
}

pub(super) use macros::{create_config, define_test};
