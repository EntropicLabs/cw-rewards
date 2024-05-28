use cosmwasm_std::{ConversionOverflowError, OverflowError, StdError};
use cw_utils::PaymentError;
use rewards_logic::RewardsError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    ConversionOverflow(#[from] ConversionOverflowError),

    #[error("{0}")]
    Payment(#[from] PaymentError),

    #[error("{0}")]
    Rewards(#[from] RewardsError),

    #[error("unauthorized")]
    Unauthorized {},

    #[error("Reward denom not on whitelist")]
    RewardNotWhitelisted {},

    #[error("Requested zero unstake")]
    ZeroUnstake {},

    #[error("Distributed zero rewards")]
    ZeroRewards {},

    #[error("User weights changed through hooks on this contract")]
    StakeDisabled {},

    #[error("Invalid incentive")]
    InvalidIncentive {},
}
