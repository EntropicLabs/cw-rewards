# cw-rewards

Contracts for various types of reward distribution mechanisms, built for use on Kujira.

- `simple-rewards`: Stake a token, get stake-weighted share of distributed rewards.
- `claiming-rewards`: Sits as a "level-2" simple contract on top of `permissioned-incentive-rewards`
- `incentive-rewards`: Stake a token, get stake-weighted share of distributed rewards + incentives with schedules.
- `permissioned-incentive-rewards`: Contract owner manually sets weights, weighted users get share of distributed rewards + incentives with schedules.
- `cw-hook-rewards`: User weights are set by external contracts following the `cw4` hook specifcation.