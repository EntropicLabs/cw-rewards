use cosmwasm_std::{coin, ensure, Coin, Decimal, StdError, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::Item;

use crate::RewardsSM;

const YEAR_SECONDS: u64 = 365 * 24 * 60 * 60;

pub const LAST_INFLATION_UPDATE: Item<Timestamp> = Item::new("last_inflation_update");
pub const INFLATION_FUNDS: Item<Coin> = Item::new("inflation_funds");

pub fn pending_inflation(
    storage: &dyn Storage,
    sm: &RewardsSM,
    rate: &Decimal,
    now: &Timestamp,
) -> StdResult<Option<(Coin, Coin)>> {
    let last_update = LAST_INFLATION_UPDATE.may_load(storage)?;
    if let Some(last_update) = last_update {
        let seconds = now.seconds() - last_update.seconds();
        let total_staked = sm.total_staked(storage)?;
        let mut inflation_amount =
            total_staked.mul_floor(rate * Decimal::from_ratio(seconds, YEAR_SECONDS));

        let mut funds_left = match INFLATION_FUNDS.may_load(storage)? {
            Some(coin) => coin,
            None => return Ok(None),
        };

        inflation_amount = inflation_amount.min(funds_left.amount);
        funds_left.amount = funds_left.amount.checked_sub(inflation_amount)?;

        Ok(Some((
            coin(inflation_amount.u128(), &funds_left.denom),
            funds_left,
        )))
    } else {
        Ok(None)
    }
}

pub fn crank(
    storage: &mut dyn Storage,
    sm: RewardsSM,
    rate: &Decimal,
    now: &Timestamp,
) -> StdResult<Vec<Coin>> {
    let pending = pending_inflation(storage, &sm, rate, now)?;
    LAST_INFLATION_UPDATE.save(storage, now)?;

    let (inflation, remaining_left) = match pending {
        Some((inflation, remaining_left)) => {
            if inflation.amount.is_zero() {
                return Ok(vec![]);
            }
            (vec![inflation], remaining_left)
        }
        None => return Ok(vec![]),
    };

    sm.distribute_rewards(storage, &inflation)?;

    if remaining_left.amount.is_zero() {
        INFLATION_FUNDS.remove(storage);
    } else {
        INFLATION_FUNDS.save(storage, &remaining_left)?;
    }

    Ok(inflation)
}

pub fn fund(storage: &mut dyn Storage, funds: Coin) -> StdResult<()> {
    match INFLATION_FUNDS.may_load(storage)? {
        Some(mut existing) => {
            ensure!(
                existing.denom == funds.denom,
                StdError::generic_err("inflation denom mismatch")
            );
            existing.amount += funds.amount;
            INFLATION_FUNDS.save(storage, &existing)?;

            Ok(())
        }
        None => {
            INFLATION_FUNDS.save(storage, &funds)?;
            Ok(())
        }
    }
}

pub fn withdraw(
    storage: &mut dyn Storage,
    sm: &RewardsSM,
    rate: &Decimal,
    now: &Timestamp,
    amount: Uint128,
) -> StdResult<Coin> {
    // Ensure that we don't withdraw more than the remaining funds, including after pending inflation
    let actual_remaining = pending_inflation(storage, sm, rate, now)?
        .map(|(_, remaining)| remaining.amount)
        .unwrap_or_default();

    // Still update using the old INFLATION_FUNDS, since we haven't actually cranked.
    let mut funds = INFLATION_FUNDS.load(storage)?;
    ensure!(
        amount.le(&actual_remaining),
        StdError::generic_err("insufficient funds to withdraw from inflation pool.")
    );

    funds.amount -= amount;
    if funds.amount.is_zero() {
        INFLATION_FUNDS.remove(storage);
    } else {
        INFLATION_FUNDS.save(storage, &funds)?;
    }

    Ok(coin(amount.u128(), &funds.denom))
}
