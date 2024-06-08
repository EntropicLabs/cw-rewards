use crate::RewardsSM;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use kujira::{
    bow::staking::{IncentiveResponse, ScheduleResponse},
    Denom, Schedule,
};

pub fn incentives<'a>() -> IndexedMap<u128, Incentive, IncentiveIndexes<'a>> {
    IndexedMap::new(
        "incentives",
        IncentiveIndexes {
            last_distributed: MultiIndex::new(
                |_, i| i.last_distributed.nanos(),
                "incentives",
                "incentives__ld",
            ),
        },
    )
}
pub struct IncentiveIndexes<'a> {
    /// Timestamp index
    pub last_distributed: MultiIndex<'a, u64, Incentive, u128>,
}

impl<'a> IndexList<Incentive> for IncentiveIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Incentive>> + '_> {
        let v: Vec<&dyn Index<Incentive>> = vec![&self.last_distributed];
        Box::new(v.into_iter())
    }
}

pub const INCENTIVE_ID: Item<Uint128> = Item::new("incentive_id");

#[cw_serde]
pub struct Incentive {
    pub id: Uint128,
    pub denom: Denom,
    pub schedule: Schedule,
    pub last_distributed: Timestamp,
}

impl Incentive {
    pub fn new(
        storage: &mut dyn Storage,
        denom: Denom,
        schedule: Schedule,
        now: &Timestamp,
    ) -> StdResult<Self> {
        let incentive_id = INCENTIVE_ID
            .may_load(storage)?
            .unwrap_or_default()
            .checked_add(Uint128::one())?;
        INCENTIVE_ID.save(storage, &incentive_id)?;

        Ok(Self {
            id: incentive_id,
            denom,
            schedule,
            last_distributed: *now,
        })
    }

    pub fn save(&self, storage: &mut dyn Storage) -> StdResult<()> {
        if self.last_distributed >= self.schedule.end {
            incentives().remove(storage, self.id.u128())
        } else {
            incentives().save(storage, self.id.u128(), self)
        }
    }

    pub fn distribute(&mut self, now: &Timestamp) -> Option<Coin> {
        let incentive_amount = self.schedule.released(&self.last_distributed, now);
        if incentive_amount.is_zero() {
            return None;
        }

        self.last_distributed = *now;
        Some(self.denom.coin(&incentive_amount))
    }
}

impl From<Incentive> for IncentiveResponse {
    fn from(i: Incentive) -> Self {
        Self {
            denom: i.denom,
            schedule: ScheduleResponse {
                start: i.schedule.start,
                end: i.schedule.end,
                release: i.schedule.release,
                amount: i.schedule.amount,
            },
        }
    }
}

pub fn load_incentives(storage: &dyn Storage, limit: usize) -> StdResult<Vec<Incentive>> {
    incentives()
        .idx
        .last_distributed
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .take(limit)
        .collect::<StdResult<Vec<_>>>()
}
pub fn distribute_lri(
    storage: &mut dyn Storage,
    limit: usize,
    sm: RewardsSM,
    now: &Timestamp,
) -> StdResult<Vec<Coin>> {
    let mut rewards = vec![];
    for mut incentive in load_incentives(storage, limit)? {
        if let Some(reward) = incentive.distribute(now) {
            rewards.push(reward);
        }
        incentive.save(storage)?;
    }
    sm.distribute_rewards(storage, &rewards)?;
    Ok(rewards)
}

pub fn get_lri(storage: &dyn Storage, limit: usize, now: &Timestamp) -> StdResult<Vec<Coin>> {
    let mut rewards = vec![];
    for mut incentive in load_incentives(storage, limit)? {
        if let Some(reward) = incentive.distribute(now) {
            rewards.push(reward);
        }
    }
    Ok(rewards)
}
