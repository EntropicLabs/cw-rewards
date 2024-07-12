use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin, Decimal, Uint128};

#[cw_serde]
pub enum Whitelist {
    All,
    Some(Vec<String>),
}

#[cw_serde]
pub enum StakingConfig {
    NativeToken { denom: String },
    Cw4Hook { cw4_addr: Addr },
    DaoDaoHook { daodao_addr: Addr },
    Permissioned {},
}

#[cw_serde]
pub struct IncentiveConfig {
    pub crank_limit: usize,
    pub min_size: Uint128,
    pub fee: Option<Coin>,
    pub whitelisted_denoms: Whitelist,
}

#[cw_serde]
pub struct DistributionConfig {
    pub fees: Vec<(Decimal, Addr)>,
    pub whitelisted_denoms: Whitelist,
}

#[cw_serde]
pub struct UnderlyingConfig {
    pub underlying_rewards_contract: Addr,
}
