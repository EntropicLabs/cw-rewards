![Entropic Labs](https://entropiclabs.io/assets/logos/entropic-name.svg)

# cw-rewards

This contract implements a flexible rewards distribution system with modular components for staking, incentives, and distribution mechanisms.

## Instantiation

To instantiate the rewards contract, you need to provide the following parameters:

```rust
pub struct InstantiateMsg {
    pub owner: Addr,
    pub staking_module: StakingConfig,
    pub incentive_module: Option<IncentiveConfig>,
    pub distribution_module: Option<DistributionConfig>,
    pub underlying_rewards_module: Option<UnderlyingConfig>,
}
```

- `owner`: The address that will have administrative privileges over the contract.
- `staking_module`: Configuration for the staking mechanism (see Modules section).
- `incentive_module`: Optional configuration for the incentives mechanism.
- `distribution_module`: Optional configuration for the direct rewards distribution mechanism.
- `underlying_rewards_module`: Optional configuration for an underlying rewards contract.

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

### Querying

The contract provides various query endpoints:

- `Config`: Returns the current contract configuration.
- `PendingRewards`: Shows the pending rewards for a given staker.
- `StakeInfo`: Provides stake information for a given staker.
- `Weights`: Lists all stakers and their weights.
- `Incentives`: Lists all active incentives.

### Admin Functions

The contract owner can:

- Update the contract configuration.
- Adjust weights directly (for `Permissioned` staking).

## Interaction with Underlying Rewards

If an underlying rewards contract is configured, this contract will automatically claim and distribute rewards from the underlying contract whenever rewards are distributed or claimed in this contract.
