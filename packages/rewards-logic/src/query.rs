use crate::{PendingRewardsResponse, StakeInfoResponse};
use cosmwasm_std::{Addr, StdResult, Storage};

use super::RewardsSM;

pub fn pending_rewards(
    sm: RewardsSM,
    storage: &dyn Storage,
    user: &Addr,
) -> StdResult<PendingRewardsResponse> {
    let pending = sm.get_accrued(storage, &user.to_string())?;
    Ok(PendingRewardsResponse { rewards: pending })
}

pub fn stake_info(
    sm: RewardsSM,
    storage: &dyn Storage,
    user: &Addr,
) -> StdResult<StakeInfoResponse> {
    let amount = sm
        .user_weights
        .may_load(storage, &user.to_string())?
        .unwrap_or_default();
    Ok(StakeInfoResponse {
        staker: user.clone(),
        amount,
    })
}
