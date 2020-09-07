# An ERC20 token contract

**NOTE:** This contract is for demonstrating ETH contracts can be implemented using CosmWasm. If you need this functionality,
use [cosmwasm-plus/cw20-base](https://github.com/CosmWasm/cosmwasm-plus/tree/master/contracts/cw20-base)
smart contract that fulfills ERC20 interface thus provides extra functionalities.

This is an implementation of Ethereum's [ERC20](https://eips.ethereum.org/EIPS/eip-20) interface.
Please note that ERC20 has some fundamental flaws, many of which have been resolved with [ERC777](https://eips.ethereum.org/EIPS/eip-777).
This projects intents to serve as a simple example that token developers can familiarize with easily, not as a modern token contract.
