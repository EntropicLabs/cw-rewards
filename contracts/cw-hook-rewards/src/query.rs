use cosmwasm_std::{Addr, Deps};

use rewards_interfaces::{PendingRewardsResponse, StakeInfoResponse};

use crate::{contract::STATE_MACHINE, ContractError};

pub fn pending_rewards(deps: Deps, staker: Addr) -> Result<PendingRewardsResponse, ContractError> {
    let accrued = STATE_MACHINE.get_accrued(deps.storage, &staker.to_string())?;
    Ok(PendingRewardsResponse { rewards: accrued })
}

pub fn stake_info(deps: Deps, staker: Addr) -> Result<StakeInfoResponse, ContractError> {
    let amount = STATE_MACHINE
        .user_weights
        .may_load(deps.storage, &staker.to_string())?
        .unwrap_or_default();
    Ok(StakeInfoResponse { staker, amount })
}
