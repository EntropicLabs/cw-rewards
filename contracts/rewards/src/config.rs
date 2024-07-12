use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Api, StdResult, Storage};
use cw_storage_plus::Item;

use rewards_interfaces::{
    modules::{DistributionConfig, IncentiveConfig, StakingConfig, UnderlyingConfig},
    msg::{ConfigResponse, ConfigUpdate, InstantiateMsg},
};

use super::ContractError;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub staking_module: StakingConfig,
    pub incentive_module: Option<IncentiveConfig>,
    pub distribution_module: Option<DistributionConfig>,
    pub underlying_rewards_module: Option<UnderlyingConfig>,
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

        if let Some(update) = msg.staking_cfg {
            self.staking_module = update.update;
        }
        if let Some(update) = msg.incentive_cfg {
            self.incentive_module = update.update;
        }
        if let Some(update) = msg.distribution_cfg {
            self.distribution_module = update.update;
        }
        if let Some(update) = msg.underlying_cfg {
            self.underlying_rewards_module = update.update;
        }

        Ok(())
    }
}

impl From<InstantiateMsg> for Config {
    fn from(msg: InstantiateMsg) -> Self {
        Self {
            owner: msg.owner,
            staking_module: msg.staking_module,
            incentive_module: msg.incentive_module,
            distribution_module: msg.distribution_module,
            underlying_rewards_module: msg.underlying_rewards_module,
        }
    }
}

impl From<Config> for ConfigResponse {
    fn from(config: Config) -> Self {
        Self {
            owner: config.owner,
            staking_module: config.staking_module,
            incentive_module: config.incentive_module,
            distribution_module: config.distribution_module,
            underlying_rewards_module: config.underlying_rewards_module,
        }
    }
}
