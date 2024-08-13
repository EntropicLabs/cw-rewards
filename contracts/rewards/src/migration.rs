use cosmwasm_schema::cw_serde;
use cosmwasm_std::{CosmosMsg, DepsMut, Env, Response, StdError};
use cw2::{get_contract_version, set_contract_version, ContractVersion};

use crate::{
    contract::CONTRACT_NAME,
    msg::{DistributionConfig, IncentiveConfig, StakingConfig, UnderlyingConfig, Whitelist},
    ContractError,
};

#[cw_serde]
pub struct MigrateMsg {}

pub fn do_migrate(
    mut deps: DepsMut,
    _env: Env,
    _msg: MigrateMsg,
) -> Result<Response, ContractError> {
    let ContractVersion {
        contract: name,
        mut version,
    } = get_contract_version(deps.storage)?;

    if name != "entropic/incentivized-rewards" && name != "entropic/cw-rewards" {
        return Err(StdError::generic_err(format!("Unexpected contract name \"{name}\"")).into());
    }

    let mut msgs = vec![];
    if version.starts_with("1.") {
        msgs.extend(migrate_1_x_x_to_2_0_0(&mut deps, &mut version)?);
    }

    if version == "2.0.0" {
        msgs.extend(migrate_2_0_0_to_2_1_0(&mut deps, &mut version)?);
    }

    Ok(Response::default().add_messages(msgs))
}

mod v_1_x_x {
    use crate::msg::Whitelist;
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
    use cw_storage_plus::Item;
    use kujira::Denom;

    #[cw_serde]
    pub struct OldConfig {
        pub owner: Addr,
        pub whitelisted_rewards: Whitelist,
        pub fees: Vec<(Decimal, Addr)>,
        pub stake_denom: Option<Denom>,
        #[serde(flatten)]
        pub incentive: Option<OldIncentiveConfig>,
        pub hook_src: Option<Addr>,
        pub underlying_rewards: Option<Addr>,
    }

    #[cw_serde]
    pub struct OldIncentiveConfig {
        pub incentive_crank_limit: usize,
        pub incentive_min: Uint128,
        pub incentive_fee: Coin,
    }

    pub const CONFIG: Item<OldConfig> = Item::new("config");
    pub const OUTPUT_VERSION: &str = "2.0.0";
}

mod v_2_0_0 {
    use crate::msg::{DistributionConfig, IncentiveConfig, StakingConfig, UnderlyingConfig};
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::Addr;
    use cw_storage_plus::Item;

    #[cw_serde]
    pub struct Config {
        pub owner: Addr,
        pub staking_module: StakingConfig,
        pub incentive_module: Option<IncentiveConfig>,
        pub distribution_module: Option<DistributionConfig>,
        pub underlying_rewards_module: Option<UnderlyingConfig>,
    }

    pub const CONFIG: Item<Config> = Item::new("config");
    pub const OUTPUT_VERSION: &str = "2.1.0";
}

pub fn migrate_1_x_x_to_2_0_0(
    deps: &mut DepsMut,
    version: &mut String,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let old_cfg = v_1_x_x::CONFIG.load(deps.storage)?;
    let staking_cfg = match (old_cfg.hook_src, old_cfg.stake_denom) {
        (None, Some(stake_denom)) => StakingConfig::NativeToken {
            denom: stake_denom.to_string(),
        },
        (Some(hook_src), None) => {
            let cw2_info = cw2::query_contract_info(&deps.querier, &hook_src)?;
            if cw2_info.contract == "crates.io:cw4-stake" {
                StakingConfig::Cw4Hook { cw4_addr: hook_src }
            } else if cw2_info.contract == "crates.io:dao-voting-token-staked" {
                StakingConfig::DaoDaoHook {
                    daodao_addr: hook_src,
                }
            } else {
                return Err(StdError::generic_err("Invalid old staking config").into());
            }
        }
        (None, None) => StakingConfig::Permissioned {},
        _ => return Err(StdError::generic_err("Invalid old staking config").into()),
    };
    let incentive_cfg = old_cfg.incentive.map(|o| IncentiveConfig {
        crank_limit: o.incentive_crank_limit,
        min_size: o.incentive_min,
        fee: Some(o.incentive_fee),
        whitelisted_denoms: Whitelist::All,
    });
    let distribution_cfg = DistributionConfig {
        whitelisted_denoms: old_cfg.whitelisted_rewards,
        fees: old_cfg.fees,
    };
    let underlying_cfg = old_cfg
        .underlying_rewards
        .map(|underlying| UnderlyingConfig {
            underlying_rewards_contract: underlying,
        });

    let new_cfg = v_2_0_0::Config {
        owner: old_cfg.owner,
        staking_module: staking_cfg,
        incentive_module: incentive_cfg,
        distribution_module: Some(distribution_cfg),
        underlying_rewards_module: underlying_cfg,
    };
    v_2_0_0::CONFIG.save(deps.storage, &new_cfg)?;

    set_contract_version(deps.storage, CONTRACT_NAME, v_1_x_x::OUTPUT_VERSION)?;
    *version = v_1_x_x::OUTPUT_VERSION.to_string();

    Ok(vec![])
}

pub fn migrate_2_0_0_to_2_1_0(
    deps: &mut DepsMut,
    version: &mut String,
) -> Result<Vec<CosmosMsg>, ContractError> {
    let old_cfg = v_2_0_0::CONFIG.load(deps.storage)?;

    let new_cfg = crate::config::Config {
        owner: old_cfg.owner,
        staking_module: old_cfg.staking_module,
        incentive_module: old_cfg.incentive_module,
        distribution_module: old_cfg.distribution_module,
        underlying_rewards_module: old_cfg.underlying_rewards_module,
        inflation_module: None,
    };

    new_cfg.save(deps.storage, deps.api)?;

    set_contract_version(deps.storage, CONTRACT_NAME, v_2_0_0::OUTPUT_VERSION)?;
    *version = v_2_0_0::OUTPUT_VERSION.to_string();

    Ok(vec![])
}
