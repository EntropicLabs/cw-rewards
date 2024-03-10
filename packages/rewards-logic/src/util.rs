use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, Decimal, Uint128};
use kujira::KujiraMsg;

pub fn calculate_total_fee(reward: &Uint128, fees: &[(Decimal, Addr)]) -> Uint128 {
    fees.iter().fold(Uint128::zero(), |acc, (fee, _)| {
        acc + reward.mul_floor(*fee)
    })
}

/// Calculates the fees sent to each address and modifies input in place.
pub fn calculate_fee_split(
    rewards: &mut Vec<Coin>,
    fees: &Vec<(Decimal, Addr)>,
) -> Vec<(Addr, Vec<Coin>)> {
    let mut result = Vec::with_capacity(fees.len());
    fees.iter().for_each(|(_, addr)| {
        result.push((addr.clone(), Vec::with_capacity(rewards.len())));
    });
    for Coin { denom, amount } in rewards.iter_mut() {
        let mut total_fee = Uint128::zero();
        for ((fee, _), (_, addr_rewards)) in fees.iter().zip(result.iter_mut()) {
            let fee_amt = amount.mul_floor(*fee);
            total_fee += fee_amt;
            if !fee_amt.is_zero() {
                addr_rewards.push(Coin {
                    denom: denom.clone(),
                    amount: fee_amt,
                });
            }
        }
        *amount -= total_fee;
    }
    rewards.retain(|c| !c.amount.is_zero());
    result.retain(|(_, coins)| !coins.is_empty());

    result
}

/// Splits the entire input amount among the recipients according to their relative weights.
pub fn calculate_fee_distribution(
    rewards: Vec<Coin>,
    fees: &Vec<(Decimal, Addr)>,
) -> Vec<(Addr, Vec<Coin>)> {
    let mut result = Vec::with_capacity(fees.len());
    let mut total_weight = Decimal::zero();

    fees.iter().for_each(|(weight, addr)| {
        total_weight += weight;
        result.push((addr.clone(), Vec::with_capacity(rewards.len())));
    });
    for Coin { denom, amount } in rewards.into_iter() {
        let mut total_fee = Uint128::zero();
        for ((fee, _), (_, addr_rewards)) in fees.iter().zip(result.iter_mut()) {
            let weight = fee / total_weight;
            let fee_amt = amount.mul_floor(weight);
            total_fee += fee_amt;
            if !fee_amt.is_zero() {
                addr_rewards.push(Coin {
                    denom: denom.clone(),
                    amount: fee_amt,
                });
            }
        }
    }

    result.retain(|(_, coins)| !coins.is_empty());

    result
}

pub fn calculate_fee_msgs(fees: Vec<(Addr, Vec<Coin>)>) -> Vec<CosmosMsg<KujiraMsg>> {
    let mut msgs = Vec::with_capacity(fees.len());
    for (addr, coins) in fees {
        if !coins.is_empty() {
            msgs.push(
                BankMsg::Send {
                    to_address: addr.into(),
                    amount: coins,
                }
                .into(),
            );
        }
    }
    msgs
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{coin, Addr, Decimal};

    #[test]
    fn test_basic_fee_split() {
        let mut rewards = vec![coin(1000, "token1"), coin(1000, "token2")];
        let fees = vec![
            (Decimal::percent(10), Addr::unchecked("test1")),
            (Decimal::percent(20), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert_eq!(result[0].0, Addr::unchecked("test1"));
        assert_eq!(result[0].1, vec![coin(100, "token1"), coin(100, "token2")]);

        assert_eq!(result[1].0, Addr::unchecked("test2"));
        assert_eq!(result[1].1, vec![coin(200, "token1"), coin(200, "token2")]);

        assert_eq!(rewards, vec![coin(700, "token1"), coin(700, "token2")]);
    }

    #[test]
    fn test_zero_fees() {
        let mut rewards = vec![coin(1000, "token1")];
        let fees = vec![(Decimal::percent(0), Addr::unchecked("test1"))];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert!(result.is_empty());

        assert_eq!(rewards, vec![coin(1000, "token1")]);
    }

    #[test]
    fn test_no_rewards() {
        let mut rewards = Vec::new();
        let fees = vec![
            (Decimal::percent(10), Addr::unchecked("test1")),
            (Decimal::percent(20), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert!(result.iter().all(|(_, coins)| coins.is_empty()));
        assert!(rewards.is_empty());
    }
    #[test]
    fn test_single_fee_recipient() {
        let mut rewards = vec![coin(1000, "token1")];
        let fees = vec![(Decimal::percent(100), Addr::unchecked("test1"))];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert_eq!(result[0].1, vec![coin(1000, "token1")]);
        assert!(rewards.is_empty());
    }

    #[test]
    fn test_high_fee_percentages() {
        let mut rewards = vec![coin(1000, "token1")];
        let fees = vec![
            (Decimal::percent(50), Addr::unchecked("test1")),
            (Decimal::percent(50), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert_eq!(result[0].1, vec![coin(500, "token1")]);
        assert_eq!(result[1].1, vec![coin(500, "token1")]);
        assert!(rewards.is_empty());
    }

    #[test]
    fn test_mixed_fee_percentages() {
        let mut rewards = vec![coin(1000, "token1")];
        let fees = vec![
            (Decimal::percent(10), Addr::unchecked("test1")),
            (Decimal::percent(30), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert_eq!(result[0].1, vec![coin(100, "token1")]);
        assert_eq!(result[1].1, vec![coin(300, "token1")]);
        assert_eq!(rewards, vec![coin(600, "token1")]);
    }

    #[test]
    fn test_rounding_effects() {
        let mut rewards = vec![coin(1000, "token1")];
        let fees = vec![
            (Decimal::from_ratio(1u128, 3u128), Addr::unchecked("test1")),
            (Decimal::from_ratio(1u128, 3u128), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert_eq!(result[0].1, vec![coin(333, "token1")]);
        assert_eq!(result[1].1, vec![coin(333, "token1")]);
        assert_eq!(rewards, vec![coin(334, "token1")]);
    }

    #[test]
    fn test_multiple_recipients_same_fee() {
        let mut rewards = vec![coin(1000, "token1")];
        let fees = vec![
            (Decimal::percent(25), Addr::unchecked("test1")),
            (Decimal::percent(25), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert_eq!(result[0].1, vec![coin(250, "token1")]);
        assert_eq!(result[1].1, vec![coin(250, "token1")]);
        assert_eq!(rewards, vec![coin(500, "token1")]);
    }

    #[test]
    fn test_multiple_recipients_one_zero() {
        let mut rewards = vec![coin(1000, "token1")];
        let fees = vec![
            (Decimal::percent(50), Addr::unchecked("test1")),
            (Decimal::percent(0), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_split(&mut rewards, &fees);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].1, vec![coin(500, "token1")]);
        assert_eq!(rewards, vec![coin(500, "token1")]);
    }

    #[test]
    fn test_distribution_equal() {
        let rewards = vec![coin(1000, "token1")];
        let fees = vec![
            (Decimal::percent(25), Addr::unchecked("test1")),
            (Decimal::percent(25), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_distribution(rewards, &fees);

        assert_eq!(result[0].1, vec![coin(500, "token1")]);
        assert_eq!(result[1].1, vec![coin(500, "token1")]);
    }

    #[test]
    fn test_distribution_unequal() {
        let rewards = vec![coin(1000, "token1")];
        let fees = vec![
            (Decimal::percent(5), Addr::unchecked("test1")),
            (Decimal::percent(15), Addr::unchecked("test2")),
        ];

        let result = calculate_fee_distribution(rewards, &fees);

        assert_eq!(result[0].1, vec![coin(250, "token1")]);
        assert_eq!(result[1].1, vec![coin(750, "token1")]);
    }
}
