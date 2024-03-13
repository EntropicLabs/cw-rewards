use cosmwasm_std::{coin, coins, Addr};
use cw_multi_test::{Contract, ContractWrapper, Executor};
use kujira::{KujiraMsg, KujiraQuery, Schedule};
use kujira_rs_testing::mock::{mock_app, CustomApp};
use rewards_interfaces::{claiming::InstantiateMsg, simple::WhitelistedRewards};

fn contract() -> Box<dyn Contract<KujiraMsg, KujiraQuery>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

fn incentive_rewards() -> Box<dyn Contract<KujiraMsg, KujiraQuery>> {
    let contract = ContractWrapper::new(
        permissioned_incentive_rewards::contract::execute,
        permissioned_incentive_rewards::contract::instantiate,
        permissioned_incentive_rewards::contract::query,
    );
    Box::new(contract)
}

pub struct Addrs {
    pub pir: Addr,
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

    let incentive_rewards_id = app.store_code(incentive_rewards());
    let rewards_id = app.store_code(contract());

    let pir_addr = app
        .instantiate_contract(
            incentive_rewards_id,
            admin.clone(),
            &rewards_interfaces::permissioned_incentive::InstantiateMsg {
                owner: admin.clone(),
                whitelisted_rewards: WhitelistedRewards::All,
                fees: vec![],
                incentive_crank_limit: 10,
                incentive_min: 100u128.into(),
                incentive_fee: coin(100u128, "TOKEN"),
            },
            &[],
            "pir",
            None,
        )
        .unwrap();

    let rewards_addr = app
        .instantiate_contract(
            rewards_id,
            admin.clone(),
            &InstantiateMsg {
                owner: admin.clone(),
                stake_denom: "TOKEN".into(),
                whitelisted_rewards: WhitelistedRewards::All,
                fees: vec![],
                underlying_rewards: pir_addr.clone(),
            },
            &[],
            "rewards",
            None,
        )
        .unwrap();

    // Set weights on PIR contract:
    app.execute_contract(
        admin.clone(),
        pir_addr.clone(),
        &rewards_interfaces::permissioned_incentive::ExecuteMsg::AdjustWeights {
            delta: vec![
                (rewards_addr.clone(), 150u128.into()),
                (user2.clone(), 50u128.into()),
            ],
        },
        &[],
    )
    .unwrap();

    (
        app,
        Addrs {
            pir: pir_addr,
            rewards: rewards_addr,
            user,
            user2,
            admin,
        },
    )
}

pub fn add_default_incentive(app: &mut CustomApp, a: &Addrs) {
    let now = app.block_info().time;
    let end = now.plus_seconds(60);
    app.execute_contract(
        a.admin.clone(),
        a.pir.clone(),
        &rewards_interfaces::permissioned_incentive::ExecuteMsg::AddIncentive {
            denom: "OTHER_TOKEN".into(),
            schedule: Schedule {
                start: now,
                end,
                amount: 1_000u128.into(),
                release: kujira::Release::Fixed,
            },
        },
        &[coin(1_000u128, "OTHER_TOKEN"), coin(100, "TOKEN")],
    )
    .unwrap();
}
