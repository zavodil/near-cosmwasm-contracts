use cosmwasm_std::{StdError, Uint64};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid Message")]
    InvalidMsg {},

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Create NFT Failed")]
    CreateFailed {},

    #[error("Cannot parse value")]
    ParseError {},

    #[error("NFT with ID {} does not exists", nft_id)]
    NFTNotExists { nft_id: Uint64 },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}
