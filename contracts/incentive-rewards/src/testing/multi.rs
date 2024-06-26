use cosmwasm_std::{coin, coins, Addr, Empty};
use cw_multi_test::{Contract, ContractWrapper, Executor};
use rewards_interfaces::{incentive::InstantiateMsg, simple::WhitelistedRewards};
use rewards_tests::{mock_app, CustomApp};

fn contract() -> Box<dyn Contract<Empty, Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub struct Addrs {
    pub rewards: Addr,
    pub user: Addr,
    pub user2: Addr,
    pub admin: Addr,
}

pub fn setup_env() -> (CustomApp, Addrs) {
    let mut app = mock_app(vec![]);
    let user = app.api().addr_make("user");
    let user2 = app.api().addr_make("user2");
    let admin = app.api().addr_make("admin");
    let balances = vec![
        (user.clone(), coins(1_000_000_000_000, "TOKEN")),
        (user2.clone(), coins(1_000_000_000_000, "TOKEN")),
        (
            admin.clone(),
            vec![
                coin(1_000_000_000_000, "TOKEN"),
                coin(1_000_000_000_000, "OTHER_TOKEN"),
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
                stake_denom: "TOKEN".into(),
                whitelisted_rewards: WhitelistedRewards::All,
                fees: vec![],
                incentive_crank_limit: 10,
                incentive_min: 100u128.into(),
                incentive_fee: coin(100u128, "TOKEN"),
            },
            &[],
            "rewards",
            None,
        )
        .unwrap();

    (
        app,
        Addrs {
            rewards: rewards_addr,
            user,
            user2,
            admin,
        },
    )
}
