use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use kujira::{bow::staking::IncentivesResponse, Schedule};
use serde::{Deserialize, Serialize};

use crate::modules::{DistributionConfig, IncentiveConfig, StakingConfig, UnderlyingConfig};

#[cw_serde]
pub enum StakeChangedHookMsg {
    Stake { addr: Addr, amount: Uint128 },
    Unstake { addr: Addr, amount: Uint128 },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub staking_module: StakingConfig,
    pub incentive_module: Option<IncentiveConfig>,
    pub distribution_module: Option<DistributionConfig>,
    pub underlying_rewards_module: Option<UnderlyingConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum ExecuteMsg {
    UpdateConfig(ConfigUpdate),
    /// Adds an incentive with the specified [`Schedule`]. Only works if incentives modules is enabled.
    AddIncentive {
        denom: String,
        schedule: Schedule,
    },
    /// Weight change hook from the DAODAO contract
    StakeChangeHook(StakeChangedHookMsg),
    /// Weight change hook from the CW4 contract
    MemberChangedHook(cw4::MemberChangedHookMsg),
    /// Manual weight change from the owner. Only works if staking module is set to Permissioned
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
    #[returns(Vec<crate::StakeInfoResponse>)]
    Weights {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    #[returns(IncentivesResponse)]
    Incentives {
        start_after: Option<Uint128>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ModuleUpdate<T> {
    pub update: T,
}

#[cw_serde]
pub struct ConfigUpdate {
    pub owner: Option<Addr>,
    pub staking_cfg: Option<ModuleUpdate<StakingConfig>>,
    pub incentive_cfg: Option<ModuleUpdate<Option<IncentiveConfig>>>,
    pub distribution_cfg: Option<ModuleUpdate<Option<DistributionConfig>>>,
    pub underlying_cfg: Option<ModuleUpdate<Option<UnderlyingConfig>>>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub staking_module: StakingConfig,
    pub incentive_module: Option<IncentiveConfig>,
    pub distribution_module: Option<DistributionConfig>,
    pub underlying_rewards_module: Option<UnderlyingConfig>,
}
