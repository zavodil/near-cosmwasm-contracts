use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::{Uint64, StdResult, StdError};
use crate::error::ContractError;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct InstantiateMsg {
    pub nft_code_id: Uint64
}

impl InstantiateMsg {
    pub fn validate(self) -> Result<(),ContractError> {
        if self.nft_code_id < 0 {
            return Err(StdError::GenericErr { msg: "incorrect code_id".to_string(), ..Default::default()}.into())
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateNft {
        name: String,
        symbol: String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
