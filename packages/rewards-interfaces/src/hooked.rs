use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};
use serde::{Deserialize, Serialize};

pub use crate::simple::{MigrateMsg, WhitelistedRewards};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub hook_src: Addr,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum ExecuteMsg {
    UpdateConfig(ConfigUpdate),
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
