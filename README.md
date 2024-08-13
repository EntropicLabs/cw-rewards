![Entropic Labs](https://entropiclabs.io/assets/logos/entropic-name.svg)

# cw-rewards

This contract implements a flexible rewards distribution system with modular components for staking, incentives, distribution mechanisms, and inflation.

## Instantiation

To instantiate the rewards contract, you need to provide the following parameters:

```rust
pub struct InstantiateMsg {
    pub owner: Addr,
    pub staking_module: StakingConfig,
    pub incentive_module: Option<IncentiveConfig>,
    pub distribution_module: Option<DistributionConfig>,
    pub underlying_rewards_module: Option<UnderlyingConfig>,
    pub inflation_module: Option<InflationConfig>,
}
```

- `owner`: The address that will have administrative privileges over the contract.
- `staking_module`: Configuration for the staking mechanism (see Modules section).
- `incentive_module`: Optional configuration for the incentives mechanism.
- `distribution_module`: Optional configuration for the direct rewards distribution mechanism.
- `underlying_rewards_module`: Optional configuration for an underlying rewards contract.
- `inflation_module`: Optional configuration for the inflation mechanism.

## Modules

The contract is composed of several modules that can be configured independently:

### 1. Staking Module

The staking module determines how users can stake their tokens. It can be configured in one of the following ways:

- `NativeToken`: Users stake native tokens directly in the contract.
- `Cw4Hook`: Staking is managed by an external CW4 group contract.
- `DaoDaoHook`: Staking is managed by a DAODAO staking contract.
- `Permissioned`: Stake weights are set directly by the contract owner.

### 2. Incentive Module

The incentive module allows for the creation of long-running incentives. It includes:

- `crank_limit`: The maximum number of incentives to process in a single operation.
- `min_size`: The minimum size of an incentive.
- `fee`: An optional fee for creating incentives.
- `whitelisted_denoms`: A whitelist of allowed denominations for incentives.

### 3. Distribution Module

The distribution module handles the direct distribution of rewards. It includes:

- `fees`: A list of fee percentages and recipient addresses, to redirect a portion of the rewards to.
- `whitelisted_denoms`: A whitelist of allowed denominations for rewards that can be directly distributed.

### 4. Underlying Rewards Module

This module allows the contract to interact with an underlying rewards contract, enabling the compounding of rewards from multiple sources.

### 5. Inflation Module

The inflation module allows for automatic generation and distribution of rewards based on a yearly inflation rate. It includes:

- `rate_per_year`: The annual inflation rate as a decimal (e.g., 0.10 for 10% per year).

## Functionality

### Staking

Users can stake tokens based on the configured staking module:

- For `NativeToken`, users send tokens directly to the contract.
- For `Cw4Hook` and `DaoDaoHook`, staking is managed by the respective external contracts.
- For `Permissioned`, the contract owner sets stake weights directly.

### Unstaking

Users can unstake their tokens, which reduces their stake weight and returns the staked tokens (for `NativeToken` staking).

### Distributing Rewards

Anyone can distribute rewards to the contract. The rewards are divided among stakers based on their stake weights, after deducting any configured fees.

### Claiming Rewards

Stakers can claim their accrued rewards at any time.

### Adding Incentives

If the incentive module is enabled, users can create long-running incentives by providing tokens and a schedule for their distribution.

### Funding Inflation

The contract owner can fund the inflation module by sending tokens to the contract using the `FundInflation` message.

### Withdrawing Inflation

The contract owner can withdraw tokens from the inflation module using the `WithdrawInflation` message.

### Inflation Distribution

Inflation rewards are automatically calculated and distributed when other contract operations (like staking or distributing rewards) are performed. The inflation rate is applied to the total staked amount, prorated for the time since the last update.

### Querying

The contract provides various query endpoints:

- `Config`: Returns the current contract configuration.
- `PendingRewards`: Shows the pending rewards for a given staker.
- `StakeInfo`: Provides stake information for a given staker.
- `Weights`: Lists all stakers and their weights.
- `Incentives`: Lists all active incentives.
- `Inflation`: Returns the current inflation rate and available funds for inflation.

### Admin Functions

The contract owner can:

- Update the contract configuration.
- Adjust weights directly (for `Permissioned` staking).
- Fund and withdraw from the inflation module.
- Enable, disable, or update the inflation module configuration.

## Interaction with Underlying Rewards

If an underlying rewards contract is configured, this contract will automatically claim and distribute rewards from the underlying contract whenever rewards are distributed or claimed in this contract.

The inflation mechanism works alongside other reward sources, providing an additional stream of rewards to stakers based on the configured annual rate.

## Example Messages

### Instantiate

```json
{
  "owner": "kujira1...",
  "staking_module": {
    "native_token": {
      "denom": "ukuji"
    }
  },
  "incentive_module": {
    "crank_limit": 10,
    "min_size": "1000000",
    "fee": {
      "amount": "100000",
      "denom": "ukuji"
    },
    "whitelisted_denoms": {
      "some": ["ukuji", "uusk"]
    }
  },
  "distribution_module": {
    "fees": [["0.01", "kujira1..."]],
    "whitelisted_denoms": {
      "all": {}
    }
  },
  "underlying_rewards_module": null,
  "inflation_module": {
    "rate_per_year": "0.05"
  }
}
```

### Execute Messages

#### Stake

Note: Attach the tokens you want to stake along with this message, if native tokens are set in the staking module.
If the staking module is not using native tokens, this will error.

```json
{
  "stake": {
    "withdraw_rewards": true
  }
}
```

#### Unstake

Note: If `withdraw_rewards` is set to `true`, the staker will also claim their pending rewards.

```json
{
  "unstake": {
    "amount": "1000000",
    "withdraw_rewards": true
  }
}
```

#### Claim Rewards

```json
{
  "claim_rewards": {}
}
```

#### Distribute Rewards

Note: Attach the reward tokens you want to distribute along with this message. These rewards will be distributed instantly.

```json
{
  "distribute_rewards": {}
}
```

#### Add Incentive

Note: Attach the incentive tokens along with this message, plus the fee tokens, if any are specified.
Also note that the start and end times are in nanoseconds, using UNIX epoch timestamps.

```json
{
  "add_incentive": {
    "denom": "ukuji",
    "schedule": {
      "start": "1625097600000000000",
      "end": "1627776000000000000",
      "amount": "1000000000",
      "release": "fixed"
    }
  }
}
```

#### Fund Inflation

Note: Attach the tokens you want to add to the inflation pool along with this message. Only the owner of the contract can fund inflation.

```json
{
  "fund_inflation": {}
}
```

#### Withdraw Inflation

Note: Only the owner of the contract can withdraw from the inflation pool.

```json
{
  "withdraw_inflation": {
    "amount": "1000000"
  }
}
```

#### Update Config

Note: Not all modules need to be updated at once. The module update uses the same structure as the instantiate message.
Specifying `null` for a module update will remove that module

```json
{
  "update_config": {
    "incentive_cfg": {
      "update": {
        "crank_limit": 20,
        "min_size": "2000000",
        "whitelisted_denoms": {
          "all": {}
        }
      }
    },
    "inflation_cfg": {
      "update": {
        "rate_per_year": "0.07"
      }
    },
    "incentive_cfg": {
      "remove": null
    }
  }
}
```

### Query Messages

#### Config

```json
{
  "config": {}
}
```

#### Pending Rewards

```json
{
  "pending_rewards": {
    "staker": "kujira1..."
  }
}
```

#### Stake Info

```json
{
  "stake_info": {
    "staker": "kujira1..."
  }
}
```

#### Weights

```json
{
  "weights": {
    "limit": 10
  }
}
```

#### Incentives

```json
{
  "incentives": {
    "limit": 10
  }
}
```

#### Inflation

```json
{
  "inflation": {}
}
```
