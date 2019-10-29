use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use snafu::{OptionExt, ResultExt};

use cosmwasm::errors::{ContractErr, ParseErr, Result, SerializeErr};
use cosmwasm::serde::{from_slice, to_vec};
use cosmwasm::storage::Storage;
use cosmwasm::types::{CosmosMsg, Params, Response};

#[derive(Serialize, Deserialize)]
pub struct InitMsg {
    // this is hex-encoded sha-256 hash of the preimage (must be 32*2 = 64 chars)
    pub hash: String,
    pub recipient: String,
    // you can set a last time or block height the contract is valid at
    // if *either* is non-zero and below current state, the contract is considered expired
    // and will be returned to the original funder
    pub end_height: i64,
    pub end_time: i64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HandleMsg {
    Release {
        // this is the preimage, must be exactly 32 bytes in hex (64 chars)
        // to release: sha256(from_hex(preimage)) == from_hex(hash)
        preimage: String,
    },
    Refund {},
}

#[derive(Serialize, Deserialize)]
pub struct State {
    // this is hex-encoded sha-256 hash of the preimage (must be 32*2 = 64 chars)
    pub hash: String,
    pub recipient: String,
    pub source: String,
    pub end_height: i64,
    pub end_time: i64,
}

impl State {
    fn is_expired(&self, params: &Params) -> bool {
        (self.end_height != 0 && params.block.height >= self.end_height)
            || (self.end_time != 0 && params.block.time >= self.end_time)
    }
}

pub static CONFIG_KEY: &[u8] = b"config";

pub fn init<T: Storage>(store: &mut T, params: Params, msg: Vec<u8>) -> Result<Response> {
    let msg: InitMsg = from_slice(&msg).context(ParseErr {})?;
    // ensure this is 32 bytes hex-encoded
    let _ = parse_hex_32(&msg.hash)?;

    let state = State {
        hash: msg.hash,
        recipient: msg.recipient,
        source: params.message.signer.clone(),
        end_height: msg.end_height,
        end_time: msg.end_time,
    };
    if state.is_expired(&params) {
        ContractErr {
            msg: "creating expired swap".to_string(),
        }
        .fail()
    } else {
        store.set(CONFIG_KEY, &to_vec(&state).context(SerializeErr {})?);
        Ok(Response::default())
    }
}

pub fn handle<T: Storage>(store: &mut T, params: Params, msg: Vec<u8>) -> Result<Response> {
    let msg: HandleMsg = from_slice(&msg).context(ParseErr {})?;
    let data = store.get(CONFIG_KEY).context(ContractErr {
        msg: "uninitialized data".to_string(),
    })?;
    let state: State = from_slice(&data).context(ParseErr {})?;

    match msg {
        HandleMsg::Release { preimage } => try_release(params, state, preimage),
        HandleMsg::Refund {} => try_refund(params, state),
    }
}

fn try_release(params: Params, state: State, preimage: String) -> Result<Response> {
    if state.is_expired(&params) {
        return ContractErr {
            msg: "swap expired".to_string(),
        }.fail();
    }

    let expected = parse_hex_32(&state.hash)?;
    let preimage = parse_hex_32(&preimage)?;
    let hash = Sha256::digest(&preimage);
    if hash.as_slice() != expected.as_slice() {
        return ContractErr {
            msg: "invalid preimage".to_string(),
        }.fail();
    }

    let res = Response {
        messages: vec![CosmosMsg::Send {
            from_address: params.contract.address,
            to_address: state.recipient,
            amount: params.contract.balance,
        }],
        log: Some("swap successful".to_string()),
        data: None,
    };
    Ok(res)
}

fn try_refund(params: Params, state: State) -> Result<Response> {
    // anyone can try to refund, as long as the contract is expired
    if !state.is_expired(&params) {
        ContractErr {
            msg: "swap not yet expired".to_string(),
        }
        .fail()
    } else {
        let res = Response {
            messages: vec![CosmosMsg::Send {
                from_address: params.contract.address,
                to_address: state.source,
                amount: params.contract.balance,
            }],
            log: Some("swap expired".to_string()),
            data: None,
        };
        Ok(res)
    }
}

fn parse_hex_32(data: &str) -> Result<Vec<u8>> {
    use std::error::Error as StdError;
    match hex::decode(data) {
        Ok(bin) => if bin.len() == 32 {
            Ok(bin)
        } else {
            ContractErr{msg: "hash must be 64 characters".to_string()}.fail()
        },
        Err(e) =>
            ContractErr{msg: format!("parsing hash: {}", e.description())}.fail(),
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm::errors::Error;
    use cosmwasm::mock::MockStorage;
    use cosmwasm::types::{Coin, coin, mock_params};

    fn preimage() -> String { hex::encode(b"this is 32 bytes exact, for you!") }
    fn real_hash() -> String { hex::encode(&Sha256::digest(&hex::decode(preimage()).unwrap())) }

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
        let mut store = MockStorage::new();
        let msg = init_msg(500, 600, real_hash());
        let params = mock_params_height("creator", &coin("1000", "earth"), &[], 450, 550);
        let res = init(&mut store, params, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's check the state
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
        let mut store = MockStorage::new();
        let msg = init_msg(1000, 600, real_hash());
        let params = mock_params_height("creator", &coin("1000", "earth"), &[], 700, 700);
        let res = init(&mut store, params, msg);
        match res {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => assert_eq!(msg, "creating expired swap".to_string()),
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn cannot_initialize_invalid_hash() {
        let mut store = MockStorage::new();
        let msg = init_msg(1000, 600, "this isn't hex no, is it?".to_string());
        let params = mock_params_height("creator", &coin("1000", "earth"), &[], 700, 700);
        let res = init(&mut store, params, msg);
        match res {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => assert_eq!(msg, "parsing hash: odd number of digits".to_string()),
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn fails_on_bad_init_data() {
        let mut store = MockStorage::new();
        let bad_msg = b"{}".to_vec();
        let params = mock_params_height("creator", &coin("1000", "earth"), &[], 876, 0);
        let res = init(&mut store, params, bad_msg);
        match res {
            Ok(_) => panic!("expected error"),
            Err(Error::ParseErr { .. }) => {}
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn handle_approve() {
        let mut store = MockStorage::new();

        // initialize the store
        let msg = init_msg(1000, 600, real_hash());
        let params = mock_params_height("creator", &coin("1000", "earth"), &[], 876, 0);
        let init_res = init(&mut store, params, msg).unwrap();
        assert_eq!(0, init_res.messages.len());

        // cannot release with bad hash
        let bad_msg = to_vec(&HandleMsg::Release {preimage: hex::encode(b"this is 3x bytes exact, for you!") }).unwrap();
        let params = mock_params_height(
            "anyone",
            &coin("0", "earth"),
            &coin("1000", "earth"),
            900,
            30,
        );
        let handle_res = handle(&mut store, params, bad_msg);
        match handle_res {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => assert_eq!(msg, "invalid preimage".to_string()),
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // cannot release it when expired
        let msg = to_vec(&HandleMsg::Release {preimage: preimage() }).unwrap();
        let params = mock_params_height(
            "anyone",
            &coin("0", "earth"),
            &coin("1000", "earth"),
            1100,
            0,
        );
        let handle_res = handle(&mut store, params, msg.clone());
        match handle_res {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => assert_eq!(msg, "swap expired".to_string()),
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // release with proper preimage, before expiration
        let params = mock_params_height(
            "random dude",
            &coin("15", "earth"),
            &coin("1000", "earth"),
            999,
            0,
        );
        let handle_res = handle(&mut store, params, msg.clone()).unwrap();
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
        let mut store = MockStorage::new();

        // initialize the store
        let msg = init_msg(1000, 0, real_hash());
        let params = mock_params_height("creator", &coin("1000", "earth"), &[], 876, 0);
        let init_res = init(&mut store, params, msg).unwrap();
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
        let handle_res = handle(&mut store, params, msg.clone());
        match handle_res {
            Ok(_) => panic!("expected error"),
            Err(Error::ContractErr { msg, .. }) => {
                assert_eq!(msg, "swap not yet expired".to_string())
            }
            Err(e) => panic!("unexpected error: {:?}", e),
        }

        // anyone can release after expiration
        let params = mock_params_height(
            "anybody",
            &coin("0", "earth"),
            &coin("1000", "earth"),
            1001,
            0,
        );
        let handle_res = handle(&mut store, params, msg.clone()).unwrap();
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
}
