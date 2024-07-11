use cosmwasm_schema::cw_serde;
use cosmwasm_std::{coin, Coin, Decimal256, Order, StdError, StdResult, Storage, Uint128, Uint256};
use cw_storage_plus::{Item, Map};
use cw_utils::NativeBalance;

#[cw_serde]
pub struct RewardInfo {
    pub user: String,
    pub denom: String,
    /// Amount of rewards accrued
    pub accrued: Uint128,
    /// Last index (S) for this denom.
    pub last_index: Decimal256,
}

impl RewardInfo {
    pub fn new(user: String, denom: String) -> Self {
        Self {
            user,
            denom,
            accrued: Uint128::zero(),
            last_index: Decimal256::zero(),
        }
    }
}

pub struct RewardsSM<'a> {
    pub total_staked: Item<Uint128>,
    pub global_indices: Map<&'a str, Decimal256>,
    pub user_weights: Map<&'a String, Uint128>,
    pub user_rewards: Map<(&'a String, &'a str), RewardInfo>,
}

impl<'a> RewardsSM<'a> {
    pub const fn new() -> Self {
        Self {
            total_staked: Item::new("rwd/ts"),
            global_indices: Map::new("rwd/gi"),
            user_weights: Map::new("rwd/uw"),
            user_rewards: Map::new("rwd/ur"),
        }
    }

    pub fn initialize(&self, storage: &mut dyn Storage) -> StdResult<()> {
        self.total_staked.save(storage, &Uint128::zero())?;
        Ok(())
    }

    fn accrued_rewards(
        &self,
        last_index: Decimal256,
        index: Decimal256,
        cur_weight: Uint128,
    ) -> Uint128 {
        let delta_index = index - last_index;
        Uint256::from(cur_weight)
            .mul_floor(delta_index)
            .try_into()
            .unwrap()
    }

    /// Returns the list of accrued rewards for the specified user.
    ///
    /// if `withdraw_accrued` is true, the user's accrued rewards will be set to zero after this operation.
    fn update_user_indices(
        &self,
        storage: &mut dyn Storage,
        user: &String,
        cur_weight: Uint128,
        new_weight: Uint128,
        withdraw_accrued: bool,
    ) -> StdResult<Vec<Coin>> {
        let global_indices = self
            .global_indices
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        let mut accrued = Vec::with_capacity(global_indices.len());
        for (denom, index) in global_indices {
            let mut reward_info = self
                .user_rewards
                .may_load(storage, (user, &denom))?
                .unwrap_or_else(|| RewardInfo::new(user.clone(), denom.clone()));

            let new_accrued = self.accrued_rewards(reward_info.last_index, index, cur_weight);
            reward_info.accrued = reward_info.accrued.checked_add(new_accrued)?;
            reward_info.last_index = index;
            accrued.push(coin(reward_info.accrued.u128(), &denom));

            if withdraw_accrued {
                reward_info.accrued = Uint128::zero();
            }
            // Prune reward info if it's empty and zero
            if new_weight.is_zero() && reward_info.accrued.is_zero() {
                self.user_rewards.remove(storage, (user, &denom));
            } else {
                self.user_rewards
                    .save(storage, (user, &denom), &reward_info)?;
            }
        }

        Ok(normalize(accrued))
    }

    pub fn total_staked(&self, storage: &dyn Storage) -> StdResult<Uint128> {
        Ok(self.total_staked.may_load(storage)?.unwrap_or_default())
    }

    /// Increase the reward weight of the specified user.
    ///
    /// Returns the list of accrued rewards for the user.
    /// If `withdraw_accrued` is true, the user's accrued rewards will be set to zero after this operation.
    pub fn increase_weight(
        &self,
        storage: &mut dyn Storage,
        user: &String,
        increment: Uint128,
        withdraw_accrued: bool,
    ) -> StdResult<Vec<Coin>> {
        let cur_weight = self
            .user_weights
            .may_load(storage, user)?
            .unwrap_or_default();
        let new_weight = cur_weight.checked_add(increment)?;

        let accrued =
            self.update_user_indices(storage, user, cur_weight, new_weight, withdraw_accrued)?;
        self.user_weights.save(storage, user, &new_weight)?;
        self.total_staked.update(storage, |total| -> StdResult<_> {
            Ok(total.checked_add(increment)?)
        })?;

        Ok(accrued)
    }

    /// Decrease the reward weight of the specified user.
    ///
    /// If the new weight is zero, the weight will be removed from storage, but reward info will be kept.
    ///
    /// Returns the list of accrued rewards for the user.
    /// If `withdraw_accrued` is true, the user's accrued rewards will be set to zero after this operation.
    pub fn decrease_weight(
        &self,
        storage: &mut dyn Storage,
        user: &String,
        decrement: Uint128,
        withdraw_accrued: bool,
    ) -> StdResult<Vec<Coin>> {
        let cur_weight = self.user_weights.load(storage, user)?;
        let new_weight = cur_weight.checked_sub(decrement)?;

        let accrued =
            self.update_user_indices(storage, user, cur_weight, new_weight, withdraw_accrued)?;
        if new_weight.is_zero() {
            self.user_weights.remove(storage, user);
        } else {
            self.user_weights.save(storage, user, &new_weight)?;
        }
        self.total_staked.update(storage, |cur| -> StdResult<_> {
            Ok(cur.checked_sub(decrement)?)
        })?;

        Ok(accrued)
    }

    /// Set the reward weight of the specified user.
    ///
    /// If the new weight is zero, the weight will be removed from storage, but reward info will be kept.
    ///
    /// Returns the list of accrued rewards for the user.
    /// If `withdraw_accrued` is true, the user's accrued rewards will be set to zero after this operation.
    pub fn set_weight(
        &self,
        storage: &mut dyn Storage,
        user: &String,
        weight: Uint128,
        withdraw_accrued: bool,
    ) -> StdResult<Vec<Coin>> {
        let cur_weight = self
            .user_weights
            .may_load(storage, user)?
            .unwrap_or_default();
        let new_weight = weight;

        let accrued =
            self.update_user_indices(storage, user, cur_weight, new_weight, withdraw_accrued)?;
        if new_weight.is_zero() {
            self.user_weights.remove(storage, user);
        } else {
            self.user_weights.save(storage, user, &new_weight)?;
        }
        self.total_staked.update(storage, |cur| -> StdResult<_> {
            if cur_weight > new_weight {
                Ok(cur.checked_sub(cur_weight - new_weight)?)
            } else {
                Ok(cur.checked_add(new_weight - cur_weight)?)
            }
        })?;

        Ok(accrued)
    }

    /// Claim the accrued rewards for the specified user, setting the accrued rewards to zero.
    pub fn claim_accrued(&self, storage: &mut dyn Storage, user: &String) -> StdResult<Vec<Coin>> {
        let cur_weight = self
            .user_weights
            .may_load(storage, user)?
            .unwrap_or_default();
        let accrued = self.update_user_indices(storage, user, cur_weight, cur_weight, true)?;
        Ok(accrued)
    }

    /// Get the list of accrued rewards for the specified user.
    pub fn get_accrued(&self, storage: &dyn Storage, user: &String) -> StdResult<Vec<Coin>> {
        let cur_weight = self
            .user_weights
            .may_load(storage, user)?
            .unwrap_or_default();
        let global_indices = self
            .global_indices
            .range(storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<_>>>()?;

        let mut accrued = Vec::with_capacity(global_indices.len());
        for (denom, index) in global_indices {
            let mut reward_info = self
                .user_rewards
                .may_load(storage, (user, &denom))?
                .unwrap_or_else(|| RewardInfo::new(user.clone(), denom.clone()));

            let new_accrued = self.accrued_rewards(reward_info.last_index, index, cur_weight);
            reward_info.accrued = reward_info.accrued.checked_add(new_accrued)?;

            accrued.push(coin(reward_info.accrued.u128(), &denom));
        }

        Ok(normalize(accrued))
    }

    /// Distribute rewards by incrementing the global index for each reward denom.
    pub fn distribute_rewards(
        &self,
        storage: &mut dyn Storage,
        rewards: &Vec<Coin>,
    ) -> StdResult<()> {
        let total_staked = self.total_staked.load(storage)?;
        if total_staked.is_zero() {
            return Err(StdError::generic_err("No staked tokens"));
        }

        for coin in rewards {
            self.global_indices
                .update(storage, &coin.denom, |index| -> StdResult<_> {
                    let mut index = index.unwrap_or_default();
                    index += Decimal256::from_ratio(coin.amount, total_staked);
                    Ok(index)
                })?;
        }

        Ok(())
    }

    /// Calculate selected users' rewards given the rewards list.
    /// Does NOT modify state, or account for accrued rewards.
    pub fn calculate_users_rewards(
        &self,
        storage: &dyn Storage,
        users: &'a Vec<String>,
        rewards: &Vec<Coin>,
    ) -> StdResult<Vec<(&'a str, Vec<Coin>)>> {
        let total_staked = self.total_staked.load(storage)?;
        let mut epehemeral_indices = Vec::with_capacity(rewards.len());
        for coin in rewards {
            epehemeral_indices.push(Decimal256::from_ratio(coin.amount, total_staked));
        }

        let mut user_rewards = Vec::with_capacity(users.len());
        for user in users {
            let cur_weight = self
                .user_weights
                .may_load(storage, user)?
                .unwrap_or_default();
            let mut user_reward = Vec::with_capacity(rewards.len());
            for (i, c) in rewards.iter().enumerate() {
                let index = epehemeral_indices[i];
                let new_accrued = self.accrued_rewards(Decimal256::zero(), index, cur_weight);
                user_reward.push(coin(new_accrued.u128(), &c.denom));
            }
            user_rewards.push((user.as_str(), normalize(user_reward)));
        }

        Ok(user_rewards)
    }

    /// Add to the accrued rewards for the specified user.
    /// Can be used to add rewards from external sources / non-uniform rewards.
    pub fn add_accrued_rewards(
        &self,
        storage: &mut dyn Storage,
        user: &String,
        rewards: &Vec<Coin>,
    ) -> StdResult<()> {
        for coin in rewards {
            let mut reward_info = self
                .user_rewards
                .may_load(storage, (user, &coin.denom))?
                .unwrap_or_else(|| RewardInfo::new(user.clone(), coin.denom.clone()));

            reward_info.accrued += coin.amount;
            self.user_rewards
                .save(storage, (user, &coin.denom), &reward_info)?;

            // If the global index for this denom is not set, initialize it to 0
            if !self.global_indices.has(storage, &coin.denom) {
                self.global_indices
                    .save(storage, &coin.denom, &Decimal256::zero())?;
            }
        }

        Ok(())
    }
}

impl<'a> Default for RewardsSM<'a> {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize(coins: Vec<Coin>) -> Vec<Coin> {
    let mut coins = NativeBalance(coins);
    coins.normalize();
    coins.into_vec()
}

#[cfg(test)]
mod test {
    use cosmwasm_std::{coin, coins, testing::mock_dependencies};

    use super::RewardsSM;

    #[test]
    fn increase_weight() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        let coins = state
            .increase_weight(deps.storage, &user, 100u128.into(), false)
            .expect("increase works");
        assert!(coins.is_empty());
        assert_eq!(
            state.user_weights.load(deps.storage, &user).unwrap().u128(),
            100u128
        );
        assert_eq!(
            state.total_staked.load(deps.storage).unwrap().u128(),
            100u128
        );
    }

    #[test]
    fn increase_weight_with_rewards() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        let ret = state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        assert!(ret.is_empty());
        assert_eq!(
            state.user_weights.load(deps.storage, &user).unwrap().u128(),
            100u128
        );
        assert_eq!(
            state.total_staked.load(deps.storage).unwrap().u128(),
            100u128
        );

        state
            .distribute_rewards(deps.storage, &coins(100u128, "ucoin"))
            .expect("distribute works");

        let ret = state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].amount.u128(), 100u128);
        assert_eq!(ret[0].denom, "ucoin");
        assert_eq!(
            state.user_weights.load(deps.storage, &user).unwrap().u128(),
            200u128
        );
        assert_eq!(
            state.total_staked.load(deps.storage).unwrap().u128(),
            200u128
        );
    }

    #[test]
    fn decrease_weight() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .increase_weight(deps.storage, &user, 100u128.into(), false)
            .expect("increase works");
        let coins = state
            .decrease_weight(deps.storage, &user, 50u128.into(), false)
            .expect("decrease works");
        assert!(coins.is_empty());
        assert_eq!(
            state.user_weights.load(deps.storage, &user).unwrap().u128(),
            50u128
        );
        assert_eq!(
            state.total_staked.load(deps.storage).unwrap().u128(),
            50u128
        );
    }

    #[test]
    fn decrease_weight_with_rewards() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        state
            .distribute_rewards(deps.storage, &coins(100u128, "ucoin"))
            .unwrap();
        let ret = state
            .decrease_weight(deps.storage, &user, 50u128.into(), true)
            .expect("decrease works");
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].amount.u128(), 100u128);
        assert_eq!(ret[0].denom, "ucoin");
        assert_eq!(
            state.user_weights.load(deps.storage, &user).unwrap().u128(),
            50u128
        );
        assert_eq!(
            state.total_staked.load(deps.storage).unwrap().u128(),
            50u128
        );
    }

    #[test]
    fn set_weight() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        let coins = state
            .set_weight(deps.storage, &user, 100u128.into(), false)
            .expect("set works");
        assert!(coins.is_empty());
        assert_eq!(
            state.user_weights.load(deps.storage, &user).unwrap().u128(),
            100u128
        );
        assert_eq!(
            state.total_staked.load(deps.storage).unwrap().u128(),
            100u128
        );
    }

    #[test]
    fn set_weight_with_rewards() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        let ret = state
            .set_weight(deps.storage, &user, 100u128.into(), true)
            .expect("set works");
        assert!(ret.is_empty());
        assert_eq!(
            state.user_weights.load(deps.storage, &user).unwrap().u128(),
            100u128
        );
        assert_eq!(
            state.total_staked.load(deps.storage).unwrap().u128(),
            100u128
        );

        state
            .distribute_rewards(deps.storage, &coins(100u128, "ucoin"))
            .expect("distribute works");

        let ret = state
            .set_weight(deps.storage, &user, 200u128.into(), true)
            .expect("set works");
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].amount.u128(), 100u128);
        assert_eq!(ret[0].denom, "ucoin");
        assert_eq!(
            state.user_weights.load(deps.storage, &user).unwrap().u128(),
            200u128
        );
        assert_eq!(
            state.total_staked.load(deps.storage).unwrap().u128(),
            200u128
        );
    }

    #[test]
    fn claim_accrued() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        state
            .distribute_rewards(deps.storage, &coins(100u128, "ucoin"))
            .unwrap();
        let ret = state
            .claim_accrued(deps.storage, &user)
            .expect("claim works");
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].amount.u128(), 100u128);
        assert_eq!(ret[0].denom, "ucoin");
    }

    #[test]
    fn claim_with_zero_accrued() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        state
            .claim_accrued(deps.storage, &user)
            .expect("claim works");
        let ret = state
            .claim_accrued(deps.storage, &user)
            .expect("claim works");
        assert!(ret.is_empty());
    }

    #[test]
    fn get_accrued() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        state
            .distribute_rewards(deps.storage, &coins(100u128, "ucoin"))
            .unwrap();
        let ret = state.get_accrued(deps.storage, &user).expect("get works");
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].amount.u128(), 100u128);
        assert_eq!(ret[0].denom, "ucoin");
    }

    #[test]
    fn calculate_users_rewards() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user1 = "user1".to_string();
        let user2 = "user2".to_string();
        state
            .increase_weight(deps.storage, &user1, 100u128.into(), true)
            .expect("increase works");
        state
            .increase_weight(deps.storage, &user2, 200u128.into(), true)
            .expect("increase works");
        // should not be reflected in accrued rewards
        state
            .distribute_rewards(deps.storage, &coins(100u128, "ucoin"))
            .unwrap();
        let users = vec![user1.clone(), user2.clone()];
        let ret = state
            .calculate_users_rewards(deps.storage, &users, &coins(100u128, "ucoin"))
            .expect("calculate works");

        assert_eq!(ret.len(), 2);
        assert_eq!(ret[0].0, user1.as_str());
        assert_eq!(ret[1].0, user2.as_str());
        assert_eq!(ret[0].1.len(), 1);
        assert_eq!(ret[1].1.len(), 1);
        assert_eq!(ret[0].1[0].amount.u128(), 33u128);
        assert_eq!(ret[1].1[0].amount.u128(), 66u128);
    }

    #[test]
    fn calculate_users_rewards_with_nonexistent_user() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user1 = "user1".to_string();
        let user2 = "user2".to_string();
        state
            .increase_weight(deps.storage, &user1, 100u128.into(), true)
            .expect("increase works");
        let users = vec![user1.clone(), user2.clone()];
        let ret = state
            .calculate_users_rewards(deps.storage, &users, &coins(100u128, "ucoin"))
            .expect("calculate works");

        assert_eq!(ret[0].0, user1.as_str());
        assert_eq!(ret[1].0, user2.as_str());
        assert_eq!(ret[0].1.len(), 1);
        assert_eq!(ret[1].1.len(), 0);
        assert_eq!(ret[0].1[0].amount.u128(), 100u128);
    }

    #[test]
    fn calculate_users_rewards_with_zero_rewards() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user1 = "user1".to_string();
        let user2 = "user2".to_string();
        state
            .increase_weight(deps.storage, &user1, 100u128.into(), true)
            .expect("increase works");
        state
            .increase_weight(deps.storage, &user2, 200u128.into(), true)
            .expect("increase works");
        let users = vec![user1.clone(), user2.clone()];
        let ret = state
            .calculate_users_rewards(deps.storage, &users, &vec![])
            .expect("calculate works");
        assert_eq!(ret.len(), 2);
        assert_eq!(ret[0].0, user1.as_str());
        assert_eq!(ret[1].0, user2.as_str());
        assert!(ret[0].1.is_empty());
        assert!(ret[1].1.is_empty());
    }

    #[test]
    fn add_accrued_rewards_zero_weight_zero_existing() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .add_accrued_rewards(deps.storage, &user, &coins(100u128, "ucoin"))
            .expect("add works");
        let ret = state.get_accrued(deps.storage, &user).expect("get works");
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].amount.u128(), 100u128);
        assert_eq!(ret[0].denom, "ucoin");
    }

    #[test]
    fn add_accrued_rewards_with_weight_zero_existing() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        state
            .add_accrued_rewards(deps.storage, &user, &coins(100u128, "ucoin"))
            .expect("add works");
        let ret = state.get_accrued(deps.storage, &user).expect("get works");
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].amount.u128(), 100u128);
        assert_eq!(ret[0].denom, "ucoin");
    }

    #[test]
    fn add_accrued_rewards_with_weight_and_existing() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        state
            .distribute_rewards(deps.storage, &coins(100u128, "ucoin"))
            .expect("distribute works");
        state
            .add_accrued_rewards(deps.storage, &user, &coins(100u128, "ucoin"))
            .expect("add works");
        state
            .add_accrued_rewards(deps.storage, &user, &coins(100u128, "ucoin"))
            .expect("add works");
        let ret = state.get_accrued(deps.storage, &user).expect("get works");
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].amount.u128(), 300u128);
        assert_eq!(ret[0].denom, "ucoin");
    }

    #[test]
    fn add_accrued_rewards_multiple_coins() {
        let mut odeps = mock_dependencies();
        let state = RewardsSM::new();
        let deps = odeps.as_mut();
        state.initialize(deps.storage).expect("initialize works");

        let user = "user".to_string();
        state
            .increase_weight(deps.storage, &user, 100u128.into(), true)
            .expect("increase works");
        state
            .distribute_rewards(deps.storage, &coins(100u128, "ucoin"))
            .expect("distribute works");
        state
            .add_accrued_rewards(
                deps.storage,
                &user,
                &vec![coin(100u128, "ucoin"), coin(200u128, "ucash")],
            )
            .expect("add works");
        let ret = state.get_accrued(deps.storage, &user).expect("get works");
        assert_eq!(ret.len(), 2);
        assert_eq!(ret[0].denom, "ucash");
        assert_eq!(ret[0].amount.u128(), 200u128);
        assert_eq!(ret[1].denom, "ucoin");
        assert_eq!(ret[1].amount.u128(), 200u128);
    }
}
