use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Coin, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use kujira::Denom;
use rewards_interfaces::incentive::{
    ConfigResponse, ConfigUpdate, InstantiateMsg, WhitelistedRewards,
};

use super::ContractError;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub stake_denom: Denom,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
    pub incentive_crank_limit: usize,
    pub incentive_min: Uint128,
    pub incentive_fee: Coin,
}

impl Config {
    pub fn load(storage: &dyn Storage) -> StdResult<Self> {
        Item::new("config").load(storage)
    }

    pub fn save(&self, storage: &mut dyn Storage, api: &dyn Api) -> Result<(), ContractError> {
        self.validate(api)?;
        Ok(Item::new("config").save(storage, self)?)
    }

    pub fn validate(&self, api: &dyn Api) -> Result<(), ContractError> {
        api.addr_validate(self.owner.as_str())?;

        Ok(())
    }

    pub fn apply_update(&mut self, msg: ConfigUpdate) -> Result<(), ContractError> {
        if let Some(owner) = msg.owner {
            self.owner = owner;
        }
        if let Some(stake_denom) = msg.stake_denom {
            self.stake_denom = stake_denom;
        }
        if let Some(whitelisted_rewards) = msg.whitelisted_rewards {
            self.whitelisted_rewards = whitelisted_rewards;
        }
        if let Some(fees) = msg.fees {
            self.fees = fees;
        }
        if let Some(incentive_crank_limit) = msg.incentive_crank_limit {
            self.incentive_crank_limit = incentive_crank_limit;
        }
        if let Some(incentive_min) = msg.incentive_min {
            self.incentive_min = incentive_min;
        }
        if let Some(incentive_fee) = msg.incentive_fee {
            self.incentive_fee = incentive_fee;
        }

        Ok(())
    }
}

impl From<InstantiateMsg> for Config {
    fn from(msg: InstantiateMsg) -> Self {
        Self {
            owner: msg.owner,
            stake_denom: msg.stake_denom,
            whitelisted_rewards: msg.whitelisted_rewards,
            fees: msg.fees,
            incentive_crank_limit: msg.incentive_crank_limit,
            incentive_min: msg.incentive_min,
            incentive_fee: msg.incentive_fee,
        }
    }
}

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> Self {
        Self {
            owner: config.owner,
            stake_denom: config.stake_denom,
            whitelisted_rewards: config.whitelisted_rewards,
            fees: config.fees,
            incentive_crank_limit: config.incentive_crank_limit,
            incentive_min: config.incentive_min,
            incentive_fee: config.incentive_fee,
        }
    }
}
