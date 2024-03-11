use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, Decimal, StdResult, Storage};
use cw_storage_plus::Item;

use kujira::Denom;
use rewards_interfaces::claiming::{
    ConfigResponse, ConfigUpdate, InstantiateMsg, WhitelistedRewards,
};

use super::ContractError;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub underlying_rewards: Addr,
    pub stake_denom: Denom,
    pub whitelisted_rewards: WhitelistedRewards,
    pub fees: Vec<(Decimal, Addr)>,
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
        if let Some(underlying_rewards) = msg.underlying_rewards {
            self.underlying_rewards = underlying_rewards;
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

        Ok(())
    }
}

impl From<InstantiateMsg> for Config {
    fn from(msg: InstantiateMsg) -> Self {
        Self {
            owner: msg.owner,
            underlying_rewards: msg.underlying_rewards,
            stake_denom: msg.stake_denom,
            whitelisted_rewards: msg.whitelisted_rewards,
            fees: msg.fees,
        }
    }
}

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> Self {
        Self {
            owner: config.owner,
            underlying_rewards: config.underlying_rewards,
            stake_denom: config.stake_denom,
            whitelisted_rewards: config.whitelisted_rewards,
            fees: config.fees,
        }
    }
}
