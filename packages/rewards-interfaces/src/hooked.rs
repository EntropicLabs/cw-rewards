use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use serde::{Deserialize, Serialize};

pub use crate::simple::{MigrateMsg, WhitelistedRewards};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub hook_src: Addr,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
    pub initialize_weights_from: Option<WeightsSource>,
}

#[cw_serde]
pub enum WeightsSource {
    DAODAO { staking: Addr },
}

#[cw_serde]
pub enum StakeChangedHookMsg {
    Stake { addr: Addr, amount: Uint128 },
    Unstake { addr: Addr, amount: Uint128 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum ExecuteMsg {
    UpdateConfig(ConfigUpdate),
    StakeChangeHook(StakeChangedHookMsg),
    MemberChangedHook(cw4::MemberChangedHookMsg),
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
}

#[cw_serde]
pub struct ConfigUpdate {
    pub owner: Option<Addr>,
    pub hook_src: Option<Addr>,
    pub whitelisted_rewards: Option<WhitelistedRewards>,
    pub fees: Option<Vec<(Decimal, Addr)>>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub hook_src: Addr,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
}
