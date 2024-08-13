use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_rewards_logic::{PendingRewardsResponse, RewardsMsg, StakeInfoResponse};
use kujira::{bow::staking::IncentivesResponse, Schedule};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Config;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub staking_module: StakingConfig,
    pub incentive_module: Option<IncentiveConfig>,
    pub distribution_module: Option<DistributionConfig>,
    pub underlying_rewards_module: Option<UnderlyingConfig>,
    pub inflation_module: Option<InflationConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum ExecuteMsg {
    UpdateConfig(ConfigUpdate),
    /// Adds an incentive with the specified [`Schedule`]. Only works if incentives modules is enabled.
    AddIncentive {
        denom: String,
        schedule: Schedule,
    },
    /// Adds the sent funds to the inflation module. Only works if inflation module is enabled, and if the
    /// sent funds are in the correct denomination, as specified in the inflation module.
    FundInflation {},
    /// Withdraw rewards from the inflation module. Only works if inflation module is enabled.
    WithdrawInflation {
        amount: Uint128,
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
    Rewards(RewardsMsg),
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Config)]
    Config {},
    #[returns(PendingRewardsResponse)]
    PendingRewards { staker: Addr },
    #[returns(StakeInfoResponse)]
    StakeInfo { staker: Addr },
    #[returns(Vec<StakeInfoResponse>)]
    Weights {
        start_after: Option<Addr>,
        limit: Option<u32>,
    },
    #[returns(IncentivesResponse)]
    Incentives {
        start_after: Option<Uint128>,
        limit: Option<u32>,
    },
    #[returns(InflationResponse)]
    Inflation {},
}

#[cw_serde]
pub enum Whitelist {
    All,
    Some(Vec<String>),
}

#[cw_serde]
pub enum StakingConfig {
    NativeToken { denom: String },
    Cw4Hook { cw4_addr: Addr },
    DaoDaoHook { daodao_addr: Addr },
    Permissioned {},
}

#[cw_serde]
pub struct IncentiveConfig {
    pub crank_limit: usize,
    pub min_size: Uint128,
    pub fee: Option<Coin>,
    pub whitelisted_denoms: Whitelist,
}

#[cw_serde]
pub struct DistributionConfig {
    pub fees: Vec<(Decimal, Addr)>,
    pub whitelisted_denoms: Whitelist,
}

#[cw_serde]
pub struct UnderlyingConfig {
    pub underlying_rewards_contract: Addr,
}

#[cw_serde]
pub struct InflationConfig {
    /// Where one year is defined as 365 * 24 * 60 * 60 seconds,
    pub rate_per_year: Decimal,
}

#[cw_serde]
pub struct ModuleUpdate<T> {
    pub update: T,
}

#[cw_serde]
#[derive(Default)]
pub struct ConfigUpdate {
    pub owner: Option<Addr>,
    pub staking_cfg: Option<ModuleUpdate<StakingConfig>>,
    pub incentive_cfg: Option<ModuleUpdate<Option<IncentiveConfig>>>,
    pub distribution_cfg: Option<ModuleUpdate<Option<DistributionConfig>>>,
    pub underlying_cfg: Option<ModuleUpdate<Option<UnderlyingConfig>>>,
    pub inflation_cfg: Option<ModuleUpdate<Option<InflationConfig>>>,
}

#[cw_serde]
pub enum StakeChangedHookMsg {
    Stake { addr: Addr, amount: Uint128 },
    Unstake { addr: Addr, amount: Uint128 },
}

#[cw_serde]
pub struct InflationResponse {
    pub rate_per_year: Decimal,
    pub funds: Option<Coin>,
}
