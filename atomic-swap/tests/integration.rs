use sha2::{Digest, Sha256};

use cosmwasm::mock::MockStorage;
use cosmwasm::serde::{from_slice, to_vec};
use cosmwasm::storage::Storage;
use cosmwasm::types::{coin, mock_params, Coin, ContractResult, CosmosMsg, Params};
use cosmwasm_vm::{call_handle, call_init, Instance};

use atomic_swap::contract::{HandleMsg, InitMsg, State, CONFIG_KEY};

/**
This integration test tries to run and call the generated wasm.
It depends on a release build being available already. You can create that with:

cargo wasm && wasm-gc ./target/wasm32-unknown-unknown/release/atomic_swap.wasm

Then running `cargo test` will validate we can properly call into that generated data.
**/
static WASM: &[u8] = include_bytes!("../../target/wasm32-unknown-unknown/release/atomic_swap.wasm");

fn preimage() -> String {
    hex::encode(b"this is 32 bytes exact, for you!")
}
fn real_hash() -> String {
    hex::encode(&Sha256::digest(&hex::decode(preimage()).unwrap()))
}

fn init_msg(height: i64, time: i64, hash: String) -> Vec<u8> {
    to_vec(&InitMsg {
        hash: hash,
        recipient: String::from("benefits"),
        end_height: height,
        end_time: time,
    })
    .unwrap()
}

fn mock_params_height(
    signer: &str,
    sent: &[Coin],
    balance: &[Coin],
    height: i64,
    time: i64,
) -> Params {
    let mut params = mock_params(signer, sent, balance);
    params.block.height = height;
    params.block.time = time;
    params
}

#[test]
fn proper_initialization() {
    let storage = MockStorage::new();
    let mut instance = Instance::from_code(&WASM, storage).unwrap();
    let msg = init_msg(500, 600, real_hash());
    let params = mock_params_height("creator", &coin("1000", "earth"), &[], 450, 550);
    let res = call_init(&mut instance, &params, &msg).unwrap().unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's check the state
    let store = instance.take_storage().unwrap();
    let data = store.get(CONFIG_KEY).expect("no data stored");
    let state: State = from_slice(&data).unwrap();
    assert_eq!(state.hash, real_hash());
    assert_eq!(state.recipient, String::from("benefits"));
    assert_eq!(state.source, String::from("creator"));
    assert_eq!(state.end_height, 500);
    assert_eq!(state.end_time, 600);
}

#[test]
fn cannot_initialize_expired() {
    let storage = MockStorage::new();
    let mut instance = Instance::from_code(&WASM, storage).unwrap();
    let msg = init_msg(1000, 600, real_hash());
    let params = mock_params_height("creator", &coin("1000", "earth"), &[], 700, 700);
    let res = call_init(&mut instance, &params, &msg).unwrap();
    match res {
        ContractResult::Ok(_) => panic!("expected error"),
        ContractResult::Err(msg) => {
            assert_eq!(msg, "Contract error: creating expired swap".to_string())
        }
    }
}

#[test]
fn cannot_initialize_invalid_hash() {
    let storage = MockStorage::new();
    let mut instance = Instance::from_code(&WASM, storage).unwrap();
    let msg = init_msg(1000, 600, "this isn't hex no, is it?".to_string());
    let params = mock_params_height("creator", &coin("1000", "earth"), &[], 700, 700);
    let res = call_init(&mut instance, &params, &msg).unwrap();
    match res {
        ContractResult::Ok(_) => panic!("expected error"),
        ContractResult::Err(msg) => assert_eq!(
            msg,
            "Contract error: parsing hash: odd number of digits".to_string()
        ),
    }
}

#[test]
fn fails_on_bad_init_data() {
    let storage = MockStorage::new();
    let mut instance = Instance::from_code(&WASM, storage).unwrap();
    let bad_msg = b"{}".to_vec();
    let params = mock_params_height("creator", &coin("1000", "earth"), &[], 876, 0);
    let res = call_init(&mut instance, &params, &bad_msg).unwrap();
    match res {
        ContractResult::Ok(_) => panic!("expected error"),
        ContractResult::Err(msg) => {
            assert_eq!(msg, "Parse error: missing field `hash`".to_string())
        }
    }
}

#[test]
fn handle_approve() {
    let storage = MockStorage::new();
    let mut instance = Instance::from_code(&WASM, storage).unwrap();

    // initialize the store
    let msg = init_msg(1000, 600, real_hash());
    let params = mock_params_height("creator", &coin("1000", "earth"), &[], 876, 0);
    let init_res = call_init(&mut instance, &params, &msg).unwrap().unwrap();
    assert_eq!(0, init_res.messages.len());

    // cannot release with bad hash
    let bad_msg = to_vec(&HandleMsg::Release {
        preimage: hex::encode(b"this is 3x bytes exact, for you!"),
    })
    .unwrap();
    let params = mock_params_height(
        "anyone",
        &coin("0", "earth"),
        &coin("1000", "earth"),
        900,
        30,
    );
    let handle_res = call_handle(&mut instance, &params, &bad_msg).unwrap();
    match handle_res {
        ContractResult::Ok(_) => panic!("expected error"),
        ContractResult::Err(msg) => assert_eq!(msg, "Contract error: invalid preimage".to_string()),
    }

    // cannot release it when expired
    let msg = to_vec(&HandleMsg::Release {
        preimage: preimage(),
    })
    .unwrap();
    let params = mock_params_height(
        "anyone",
        &coin("0", "earth"),
        &coin("1000", "earth"),
        1100,
        0,
    );
    let handle_res = call_handle(&mut instance, &params, &msg).unwrap();
    match handle_res {
        ContractResult::Ok(_) => panic!("expected error"),
        ContractResult::Err(msg) => assert_eq!(msg, "Contract error: swap expired".to_string()),
    }

    // release with proper preimage, before expiration
    let params = mock_params_height(
        "random dude",
        &coin("15", "earth"),
        &coin("1000", "earth"),
        999,
        0,
    );
    let handle_res = call_handle(&mut instance, &params, &msg).unwrap().unwrap();
    assert_eq!(1, handle_res.messages.len());
    let msg = handle_res.messages.get(0).expect("no message");
    match &msg {
        CosmosMsg::Send {
            from_address,
            to_address,
            amount,
        } => {
            assert_eq!("cosmos2contract", from_address);
            assert_eq!("benefits", to_address);
            assert_eq!(1, amount.len());
            let coin = amount.get(0).expect("No coin");
            assert_eq!(coin.denom, "earth");
            assert_eq!(coin.amount, "1000");
        }
        _ => panic!("Unexpected message type"),
    }
}

#[test]
fn handle_refund() {
    let storage = MockStorage::new();
    let mut instance = Instance::from_code(&WASM, storage).unwrap();

    // initialize the store
    let msg = init_msg(1000, 0, real_hash());
    let params = mock_params_height("creator", &coin("1000", "earth"), &[], 876, 0);
    let init_res = call_init(&mut instance, &params, &msg).unwrap().unwrap();
    assert_eq!(0, init_res.messages.len());

    // cannot release when unexpired
    let msg = to_vec(&HandleMsg::Refund {}).unwrap();
    let params = mock_params_height(
        "anybody",
        &coin("0", "earth"),
        &coin("1000", "earth"),
        800,
        0,
    );
    let handle_res = call_handle(&mut instance, &params, &msg).unwrap();
    match handle_res {
        ContractResult::Ok(_) => panic!("expected error"),
        ContractResult::Err(msg) => {
            assert_eq!(msg, "Contract error: swap not yet expired".to_string())
        }
    }

    // anyone can release after expiration
    let params = mock_params_height(
        "anybody",
        &coin("0", "earth"),
        &coin("1000", "earth"),
        1001,
        0,
    );
    let handle_res = call_handle(&mut instance, &params, &msg).unwrap().unwrap();
    assert_eq!(1, handle_res.messages.len());
    let msg = handle_res.messages.get(0).expect("no message");
    match &msg {
        CosmosMsg::Send {
            from_address,
            to_address,
            amount,
        } => {
            assert_eq!("cosmos2contract", from_address);
            assert_eq!("creator", to_address);
            assert_eq!(1, amount.len());
            let coin = amount.get(0).expect("No coin");
            assert_eq!(coin.denom, "earth");
            assert_eq!(coin.amount, "1000");
        }
        _ => panic!("Unexpected message type"),
    }
}
