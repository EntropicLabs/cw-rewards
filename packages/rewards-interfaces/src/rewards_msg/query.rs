use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Uint128};

#[cw_serde]
pub struct PendingRewardsResponse {
    pub rewards: Vec<Coin>,
}

#[cw_serde]
pub struct StakeInfoResponse {
    pub staker: Addr,
    pub amount: Uint128,
}
