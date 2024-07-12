use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Uint128};
use kujira::CallbackData;

#[cw_serde]
pub enum RewardsMsg {
    /// Stake some tokens on this contract to receive rewards.
    Stake(StakeMsg),
    /// Unstake some tokens from this contract.
    Unstake(UnstakeMsg),
    /// Claim accrued rewards based on current stake weight.
    ClaimRewards(ClaimRewardsMsg),
    /// Distribute rewards to stakers.
    DistributeRewards(DistributeRewardsMsg),
}

#[cw_serde]
pub struct StakeMsg {
    pub withdraw_rewards: bool,
    pub callback: Option<CallbackData>,
}

#[cw_serde]
pub struct UnstakeMsg {
    pub amount: Uint128,
    pub withdraw_rewards: bool,
    pub callback: Option<CallbackData>,
}

#[cw_serde]
pub struct ClaimRewardsMsg {
    pub callback: Option<CallbackData>,
}

#[cw_serde]
pub struct DistributeRewardsMsg {
    pub callback: Option<CallbackData>,
}

#[cw_serde]
pub struct PendingRewardsResponse {
    pub rewards: Vec<Coin>,
}

#[cw_serde]
pub struct StakeInfoResponse {
    pub staker: Addr,
    pub amount: Uint128,
}

impl From<StakeMsg> for RewardsMsg {
    fn from(val: StakeMsg) -> Self {
        RewardsMsg::Stake(val)
    }
}

impl From<UnstakeMsg> for RewardsMsg {
    fn from(val: UnstakeMsg) -> Self {
        RewardsMsg::Unstake(val)
    }
}

impl From<ClaimRewardsMsg> for RewardsMsg {
    fn from(val: ClaimRewardsMsg) -> Self {
        RewardsMsg::ClaimRewards(val)
    }
}

impl From<DistributeRewardsMsg> for RewardsMsg {
    fn from(val: DistributeRewardsMsg) -> Self {
        RewardsMsg::DistributeRewards(val)
    }
}
