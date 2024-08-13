pub mod contract;

mod config;
mod error;
mod execute;
mod query;
mod migration;
#[cfg(test)]
mod testing;

pub mod msg;

pub use crate::{
    config::Config,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
};
pub use cw_rewards_logic::{
    ClaimRewardsMsg, DistributeRewardsMsg, RewardsMsg, StakeMsg, UnstakeMsg,
};
