use std::fmt::Debug;

use cosmwasm_std::{Addr, Coin, Empty, StdResult, Timestamp, Uint128};
use cw_multi_test::{App, AppResponse, Contract, ContractWrapper, Executor};
use cw_utils::NativeBalance;
use kujira::Schedule;
use rewards_interfaces::modules::{
    DistributionConfig, IncentiveConfig, StakingConfig, UnderlyingConfig,
};
use rewards_interfaces::msg::{ConfigUpdate, ExecuteMsg, InstantiateMsg, QueryMsg};
use rewards_interfaces::{
    ClaimRewardsMsg, DistributeRewardsMsg, PendingRewardsResponse, RewardsMsg, StakeInfoResponse,
    StakeMsg, UnstakeMsg,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct TestEnv {
    pub app: App,
    pub owner: Addr,
    pub rewards_addr: Addr,
    pub rewards_code_id: u64,
    pub cw4_code_id: u64,
}

pub fn contract_rewards() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    Box::new(contract)
}

pub fn contract_cw4() -> Box<dyn Contract<Empty>> {
    let contract = ContractWrapper::new(
        cw4_stake::contract::execute,
        cw4_stake::contract::instantiate,
        cw4_stake::contract::query,
    );
    Box::new(contract)
}

pub fn multi_app() -> App {
    App::default()
}

pub fn setup_test_env(
    mut app: App,
    initial_balance: Vec<(&str, Vec<Coin>)>,
    instantiate_msg: InstantiateMsg,
) -> TestEnv {
    let owner = app.api().addr_make("owner");
    let rewards_code_id = app.store_code(contract_rewards());
    let cw4_code_id = app.store_code(contract_cw4());

    let initial_balance = initial_balance
        .into_iter()
        .map(|(addr, coins)| (app.api().addr_make(addr), coins))
        .collect::<Vec<_>>();

    app.init_modules(|router, _, storage| {
        // Set initial balances
        for (addr, balance) in initial_balance {
            router.bank.init_balance(storage, &addr, balance).unwrap();
        }
    });

    // Instantiate contract
    let rewards_addr = app
        .instantiate_contract(
            rewards_code_id,
            owner.clone(),
            &instantiate_msg,
            &[],
            "rewards",
            None,
        )
        .unwrap();

    TestEnv {
        app,
        owner,
        rewards_addr,
        rewards_code_id,
        cw4_code_id,
    }
}

pub fn create_config(
    app: &App,
    owner: &str,
    staking_module: StakingConfig,
    incentive_module: Option<IncentiveConfig>,
    distribution_module: Option<DistributionConfig>,
    underlying_rewards_module: Option<UnderlyingConfig>,
) -> InstantiateMsg {
    InstantiateMsg {
        owner: app.api().addr_make(owner),
        staking_module,
        incentive_module,
        distribution_module,
        underlying_rewards_module,
    }
}

impl TestEnv {
    pub fn addr(&self, account: &str) -> Addr {
        self.app.api().addr_make(account)
    }

    pub fn execute<T: Serialize + Debug>(
        &mut self,
        account: &str,
        contract: &Addr,
        msg: T,
        funds: Vec<Coin>,
    ) -> anyhow::Result<AppResponse> {
        self.app
            .execute_contract(self.addr(account), contract.clone(), &msg, &funds)
    }

    pub fn stake(&mut self, account: &str, amount: Coin) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            self.addr(account),
            self.rewards_addr.clone(),
            &ExecuteMsg::Rewards(RewardsMsg::Stake(StakeMsg {
                callback: None,
                withdraw_rewards: false,
            })),
            &[amount],
        )
    }

    pub fn unstake(&mut self, account: &str, amount: u128) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            self.addr(account),
            self.rewards_addr.clone(),
            &ExecuteMsg::Rewards(RewardsMsg::Unstake(UnstakeMsg {
                amount: Uint128::new(amount),
                callback: None,
                withdraw_rewards: false,
            })),
            &[],
        )
    }

    pub fn claim_rewards(&mut self, account: &str) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            self.addr(account),
            self.rewards_addr.clone(),
            &ExecuteMsg::Rewards(RewardsMsg::ClaimRewards(ClaimRewardsMsg { callback: None })),
            &[],
        )
    }

    pub fn distribute_rewards(
        &mut self,
        account: &str,
        amounts: Vec<Coin>,
    ) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            self.addr(account),
            self.rewards_addr.clone(),
            &ExecuteMsg::Rewards(RewardsMsg::DistributeRewards(DistributeRewardsMsg {
                callback: None,
            })),
            &amounts,
        )
    }

    pub fn assert_balance(&self, account: &str, expected: Coin) {
        let balance = self
            .app
            .wrap()
            .query_balance(self.addr(account), &expected.denom)
            .unwrap();
        assert_eq!(
            balance, expected,
            "Balance mismatch for {account}: {balance} != {expected}"
        );
    }

    pub fn assert_stake(&self, account: &str, expected: u128) {
        let stake_info: StakeInfoResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.rewards_addr.clone(),
                &QueryMsg::StakeInfo {
                    staker: self.addr(account),
                },
            )
            .unwrap();
        assert_eq!(
            stake_info.amount,
            Uint128::new(expected),
            "Stake mismatch for {}",
            account
        );
    }

    pub fn assert_pending_rewards(&self, account: &str, expected: Vec<Coin>) {
        let pending_rewards: PendingRewardsResponse = self
            .app
            .wrap()
            .query_wasm_smart(
                self.rewards_addr.clone(),
                &QueryMsg::PendingRewards {
                    staker: self.addr(account),
                },
            )
            .unwrap();
        let mut normalized = NativeBalance(expected);
        normalized.normalize();
        assert_eq!(
            pending_rewards.rewards,
            normalized.into_vec(),
            "Pending rewards mismatch for {}",
            account
        );
    }

    pub fn add_incentive(
        &mut self,
        account: &str,
        denom: &str,
        schedule: Schedule,
        payment: Vec<Coin>,
    ) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            self.addr(account),
            self.rewards_addr.clone(),
            &ExecuteMsg::AddIncentive {
                denom: denom.to_string(),
                schedule,
            },
            &payment,
        )
    }

    pub fn adjust_weights(
        &mut self,
        account: &str,
        weights: Vec<(&str, Uint128)>,
    ) -> anyhow::Result<AppResponse> {
        let weights = weights
            .into_iter()
            .map(|(addr, weight)| (self.addr(addr), weight))
            .collect();
        self.app.execute_contract(
            self.addr(account),
            self.rewards_addr.clone(),
            &ExecuteMsg::AdjustWeights { delta: weights },
            &[],
        )
    }

    pub fn update_config(
        &mut self,
        account: &str,
        update: ConfigUpdate,
    ) -> anyhow::Result<AppResponse> {
        self.app.execute_contract(
            self.addr(account),
            self.rewards_addr.clone(),
            &ExecuteMsg::UpdateConfig(update),
            &[],
        )
    }

    pub fn block_time(&self) -> Timestamp {
        self.app.block_info().time
    }

    pub fn advance_time(&mut self, seconds: u64) {
        self.app.update_block(|block| {
            block.time = block.time.plus_seconds(seconds);
            block.height += seconds / 5; // Assume 5 second block time
        });
    }

    pub fn instantiate<T: Serialize + Debug>(
        &mut self,
        msg: T,
        code_id: u64,
        label: &str,
    ) -> anyhow::Result<Addr> {
        let rewards_addr =
            self.app
                .instantiate_contract(code_id, self.owner.clone(), &msg, &[], label, None)?;
        Ok(rewards_addr)
    }

    pub fn query<T: DeserializeOwned>(&self, query_msg: QueryMsg) -> StdResult<T> {
        self.app
            .wrap()
            .query_wasm_smart(&self.rewards_addr, &query_msg)
    }
}
