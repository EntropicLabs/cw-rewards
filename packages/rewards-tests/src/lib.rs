use cosmwasm_std::{testing::MockStorage, Addr, Coin, Empty};
use cw_multi_test::{
    App, BankKeeper, BasicAppBuilder, DistributionKeeper, FailingModule, MockApiBech32,
    StakeKeeper, WasmKeeper,
};

pub type CustomApp = App<
    BankKeeper,
    MockApiBech32,
    MockStorage,
    FailingModule<Empty, Empty, Empty>,
    WasmKeeper<Empty, Empty>,
    StakeKeeper,
    DistributionKeeper,
>;

pub fn mock_app(balances: Vec<(Addr, Vec<Coin>)>) -> CustomApp {
    BasicAppBuilder::new_custom()
        .with_api(MockApiBech32::new("kujira"))
        .with_wasm(WasmKeeper::default())
        .build(|router, _, storage| {
            for (addr, coins) in balances {
                router.bank.init_balance(storage, &addr, coins).unwrap();
            }
        })
}
