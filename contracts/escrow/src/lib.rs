use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    env, near_bindgen, AccountId, Balance, BlockHeight, PanicOnDefault, Promise, PromiseOrValue,
};

pub mod contract;
mod error;
pub mod state;

use crate::state::*;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    arbiter: AccountId,
    recipient: AccountId,
    source: AccountId,
    end_height: Option<BlockHeight>,
    end_time: Option<BlockHeight>,
}
