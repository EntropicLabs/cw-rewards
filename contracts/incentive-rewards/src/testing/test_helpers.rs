use cosmwasm_std::{coin, Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};
use rewards_interfaces::modules::{DistributionConfig, IncentiveConfig, StakingConfig, Whitelist};
use rewards_interfaces::msg::InstantiateMsg;
use rewards_tests::{mock_app, CustomApp};

fn contract() -> Box<dyn Contract<Empty, Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub struct TestEnv {
    pub app: CustomApp,
    pub rewards_addr: Addr,
    pub user1: Addr,
    pub user2: Addr,
    pub admin: Addr,
}

pub fn setup_test_env() -> TestEnv {
    let mut app = mock_app(vec![]);
    let user1 = app.api().addr_make("user1");
    let user2 = app.api().addr_make("user2");
    let admin = app.api().addr_make("admin");

    let balances = vec![
        (
            user1.clone(),
            vec![
                coin(1_000_000, "TOKEN"),
                coin(1_000_000, "REWARD"),
                coin(1_000_000, "SMALL_REWARD"),
            ],
        ),
        (
            user2.clone(),
            vec![
                coin(1_000_000, "TOKEN"),
                coin(1_000_000, "REWARD"),
                coin(1_000_000, "SMALL_REWARD"),
            ],
        ),
        (
            admin.clone(),
            vec![
                coin(2_000_000, "TOKEN"),
                coin(1_000_000, "REWARD"),
                coin(1_000_000, "SMALL_REWARD"),
            ],
        ),
    ];

    app.init_modules(|router, _, storage| {
        for (account, amount) in balances.into_iter() {
            router.bank.init_balance(storage, &account, amount).unwrap();
        }
    });

    let rewards_id = app.store_code(contract());

    let rewards_addr = app
        .instantiate_contract(
            rewards_id,
            admin.clone(),
            &InstantiateMsg {
                owner: admin.clone(),
                staking_module: StakingConfig::NativeToken {
                    denom: "TOKEN".to_string(),
                },
                incentive_module: Some(IncentiveConfig {
                    crank_limit: 10,
                    min_size: 100u128.into(),
                    fee: Some(coin(100, "TOKEN")),
                    whitelisted_denoms: Whitelist::All,
                }),
                distribution_module: Some(DistributionConfig {
                    fees: vec![],
                    whitelisted_denoms: Whitelist::All,
                }),
                underlying_rewards_module: None,
            },
            &[],
            "rewards",
            None,
        )
        .unwrap();

    TestEnv {
        app,
        rewards_addr,
        user1,
        user2,
        admin,
    }
}
