![cosmwasm-near](https://user-images.githubusercontent.com/38926321/171194185-c3abea0d-d476-4468-ad63-848690aa5200.png)
Escrow Contract from [CosmWasm examples catalog](https://github.com/InterWasm/cw-contracts/tree/main/contracts/escrow) built on [NEAR Blockchain](https://near.org).

**What do NEAR and CosmWasm Runtimes have in common?**

- Both are using Rust / WebAssembly
- Key-value storage for the Contract state
- JSON objects to input and output data

**What is the difference between NEAR and CosmWasm implementation?**

<table>
<tr>
<td align="center"> 

**CosmWasm**

</td>
<td align="center">

**NEAR**

</td>
</tr>
<tr>
<td colspan="2" align="center">

**Specify public entry point**

</td>
</tr>

<tr>
<td> 

Place macros ```#[entry_point]``` before every function 


```rust
#[entry_point]
pub fn execute1() {
	...
}

#[entry_point]
pub fn execute2() {
	...
}
```

</td>
<td>

Specify function as public after [#[near_binden]](https://www.near-sdk.io/contract-structure/near-bindgen) macros.


```rust
#[near_bindgen]
impl Contract {
	pub fn execute1() {
		...
	}
	pub fn execute2() {
		...
	}
}
```


</td>
</tr>

<tr>
<td colspan="2" align="center">

**Read/write contract state**

</td>
</tr>
<tr valign="top">
<td>

Import ``schemars::JsonSchema`` to work with JSON objects.
Create public structure containing objects you need, initialize with the unique key and use functions to read and overwrite the state.  

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

static CONFIG_KEY: &[u8] = b"config";

// set object with the `arbiter` field 
#[derive(Serialize, Deserialize, Clone, 
	Debug, PartialEq, JsonSchema)]
pub struct State {
	pub arbiter: Addr,
}

// output `arbiter` field
#[entry_point]
pub fn query_arbiter(deps: Deps, _env: Env,
					 msg: QueryMsg) 
	-> StdResult<ArbiterResponse> {
	let state = 
		config_read(deps.storage).load()?;
	let addr = state.arbiter;
	Ok(ArbiterResponse { arbiter: addr })
}

pub fn config(storage: &mut dyn Storage) 
	-> Singleton<State> {
	singleton(storage, CONFIG_KEY)
}

pub fn config_read(storage: &dyn Storage) 
	-> ReadonlySingleton<State> {
	singleton_read(storage, CONFIG_KEY)
}

```

</td>
<td valign="top">

Import ``near_sdk::borsh`` to work with JSON objects. Create public structure containing objects you need, initialize it by default or with the constructor. Access with `self` parameter. 

```rust 
use near_sdk::borsh::{self, 
	BorshDeserialize, BorshSerialize};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
// set object with the `arbiter` field 
pub struct Contract {
	arbiter: AccountId,
}

// output `arbiter` field
pub fn query_arbiter(&self) -> AccountId {
	self.arbiter.clone()
}

```

</td>
</tr>

<tr valign="top">
<td colspan="2" align="center">

**Run functions with parameters**

</td>
</tr>
<tr valign="top">
<td>
Specify message templates and handle function parameters:

- `deps` allows us to perform storage related actions, validate addresses and query other smart contracts
- `_env` is mainly used to access details about the current state of the blockchain 
- `info` provides access to the message metadata (i.e., sender address, the amount and type of funds)
- `msg` is the `MsgInstantiateContract` payload, which comprises the data received from the contract creator in JSON format that conforms to the `InstantiateMsg` struct

Parse `msg` and run corresponding action.

```rust
// set message structure
pub enum ExecuteMsg {
	Approve {
		quantity: Option<Vec<Coin>>,
	},
	Refund {},
}

// create an entry point
#[entry_point]
pub fn execute(
	deps: DepsMut,
	env: Env,
	info: MessageInfo,
	msg: ExecuteMsg,
) -> Result<Response, ContractError> {
	let state = 
		config_read(deps.storage).load()?;
	// parse message
	match msg {
		ExecuteMsg::Refund {} => 
			try_refund(deps, env, 
					   info, state),
		ExecuteMsg::Approve {quantity} => 
			try_approve(deps, env, state, 
				info, quantity),
	}
}

// execute actions with given parameters
fn try_refund(
	deps: DepsMut,
	env: Env,
	_info: MessageInfo,
	state: State,
) -> Result<Response, ContractError> {
	let balance = deps.querier
		.query_all_balances(
			&env.contract.address)?;
	Ok(send_tokens(state.source, 
				   balance, "refund"))
}

fn try_approve(
	deps: DepsMut,
	env: Env,
	state: State,
	info: MessageInfo,
	quantity: Option<Vec<Coin>>,
) -> Result<Response, ContractError> {
	let amount = 
		if let Some(quantity) = quantity {
			quantity
		} else {
			// release everything
			deps.querier
			.query_all_balances(
				&env.contract.address)?
	};

	Ok(send_tokens(state.recipient, 
				   amount, "approve"))
}
```

</td>
<td>
Create a public function, perform actions needed.

```rust
// execute actions
pub fn try_refund(&self) -> Promise {
	let balance = env::account_balance();
	send_tokens(self.source.clone(), balance)
}

pub fn try_approve(&self,
                   quantity: Option<Balance>)
	-> Promise {
	let amount = 
		if let Some(quantity) = quantity {
			quantity
		} else {
			// release everything
			env::account_balance()
		};

	send_tokens(self.recipient.clone(), amount)
}
```

</td>
</tr>

<tr valign="top">
<td colspan="2" align="center">

**Initialize contract state**
</td>
</tr>
<tr valign="top">
<td>

Create `instantiate()` function as the first entry-point, introduce a new variable named `state` of type `State`, fill it with the function parameters and save.

```rust 
#[entry_point]
pub fn instantiate(
	deps: DepsMut,
	env: Env,
	info: MessageInfo,
	msg: InstantiateMsg,
) -> Result<Response, ContractError> {
	let state = State {
		arbiter: deps.api
			.addr_validate(&msg.arbiter)?,
		recipient: deps.api
			.addr_validate(&msg.recipient)?,
		source: info.sender,
		end_height: msg.end_height,
		end_time: msg.end_time,
	};

	config(deps.storage).save(&state)?;
	Ok(Response::default())
}
```

</td>
<td>

Create a function with macros `#[init]`, hande function parameters and set the contract state.

```rust 
#[init]
pub fn instantiate(
    arbiter: AccountId,
    recipient: AccountId,
    end_height: Option<BlockHeight>,
    end_time: Option<BlockHeight>,
) -> Self {
    Self {
        arbiter,
        recipient,
        source: env::predecessor_account_id(),
        end_height,
        end_time,
    }
}

```
</td>

</tr>

</table>


# Escrow Example Contract

This is a simple single-use escrow contract. It creates a contract that can hold some
native tokens and gives the power to an arbiter to release them to a pre-defined
beneficiary. They can release all tokens, or only a fraction. If an optional
timeout is reached, the tokens can no longer be released, rather they can only
be returned to the original funder. Tokens can be added to the contract at any
time without causing any errors, or losing access to them.

This contract is mainly considered as a simple tutorial example. In the real
world, you would probably want one contract to manage many escrows and allow
some global configuration options on it. It is generally simpler to rely on
some well-known address for handling all escrows securely than checking each
deployed escrow is using the proper wasm code.

- CosmWasm example: https://github.com/InterWasm/cw-contracts/tree/main/contracts/escrow
- NEAR example: https://github.com/zavodil/near-cosmwasm-contracts/tree/main/contracts/escrow

## How to deploy this contract on NEAR 

- Install [Rust](https://docs.near.org/docs/develop/contracts/rust/intro#installing-the-rust-toolchain)
- Install [NEAR CLI](https://docs.near.org/docs/develop/contracts/rust/intro#installing-the-near-cli)
- Navigate to escrow example folder and compile the code `cargo build --target wasm32-unknown-unknown --release`. Run tests: `cargo test`
- Deploy to the NEAR testnet and initialize: 

 ```near dev-deploy --wasmFile=target/wasm32-unknown-unknown/release/near_cw_escrow.wasm --initFunction instantiate --initArgs '{"arbiter": "your_arbiter_account.testnet", "recipient": "your_recipient_account.testnet"}'```
  
- This creates new account like [dev-1653949613097-64928213583496](https://explorer.testnet.near.org/accounts/dev-1653949613097-64928213583496) with some tokens inside and deploy wasm file.
- Query contract with commands like `near view dev-1653949613097-64928213583496 query_arbiter '{}'`
- Check [NEAR CLI view/call methods documentation](https://docs.near.org/docs/tools/near-cli#near-call) for more details.

## How to get help:

- NEAR-native examples: https://near.dev (contract + UI)
- Dev documentation: https://docs.near.org/
- Developer Office Hours https://near.org/office-hours/
  
## Rough overview on [contract.rs](https://github.com/zavodil/near-cosmwasm-contracts/blob/main/contracts/escrow/src/contract.rs) updates. 
  ![cosmwasm-near](https://user-images.githubusercontent.com/38926321/171194512-d74e1718-dee7-4596-ac6f-c3ea78d09839.png)
