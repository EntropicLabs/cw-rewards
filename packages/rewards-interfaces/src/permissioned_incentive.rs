use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use kujira::{bow::staking::IncentivesResponse, Denom, Schedule};
use serde::{Deserialize, Serialize};

pub use crate::simple::{MigrateMsg, WhitelistedRewards};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
    pub incentive_crank_limit: usize,
    pub incentive_min: Uint128,
    pub incentive_fee: Coin,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum ExecuteMsg {
    UpdateConfig(ConfigUpdate),
    AddIncentive {
        denom: Denom,
        schedule: Schedule,
    },
    AdjustWeights {
        delta: Vec<(Addr, Uint128)>,
    },
    /// Rewards interfaces
    #[serde(untagged)]
    Rewards(crate::RewardsMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(crate::PendingRewardsResponse)]
    PendingRewards { staker: Addr },
    #[returns(crate::StakeInfoResponse)]
    StakeInfo { staker: Addr },
    #[returns(IncentivesResponse)]
    Incentives {
        start_after: Option<Uint128>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigUpdate {
    pub owner: Option<Addr>,
    pub whitelisted_rewards: Option<WhitelistedRewards>,
    pub fees: Option<Vec<(Decimal, Addr)>>,
    pub incentive_crank_limit: Option<usize>,
    pub incentive_min: Option<Uint128>,
    pub incentive_fee: Option<Coin>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
    pub incentive_crank_limit: usize,
    pub incentive_min: Uint128,
    pub incentive_fee: Coin,
}
