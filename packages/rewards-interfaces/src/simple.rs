use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal};
use kujira::Denom;
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct MigrateMsg {}

#[cw_serde]
pub enum WhitelistedRewards {
    All,
    Some(Vec<Denom>),
}

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub stake_denom: Denom,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum ExecuteMsg {
    UpdateConfig(ConfigUpdate),
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
    pub stake_denom: Option<Denom>,
    pub whitelisted_rewards: Option<WhitelistedRewards>,
    pub fees: Option<Vec<(Decimal, Addr)>>,
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub stake_denom: Denom,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
}
