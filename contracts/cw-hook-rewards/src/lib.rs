pub mod contract;

mod config;
mod error;
mod execute;
mod query;

// FIXME: Tests
// #[cfg(test)]
// mod testing;

pub use crate::{config::Config, error::ContractError};
