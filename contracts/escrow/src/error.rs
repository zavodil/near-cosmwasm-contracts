use near_sdk::serde::Serialize;

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, PartialEq, Serialize)]
#[serde(crate = "near_sdk::serde")]
pub enum ContractError {
    Unauthorized,
    Expired {
        end_height: Option<u64>,
        end_time: Option<u64>,
    },
    NotExpired,
}

impl Display for ContractError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            ContractError::Unauthorized => write!(f, "Unauthorized"),
            ContractError::Expired {
                end_height,
                end_time,
            } => write!(
                f,
                "Escrow expired (end_height {} end_time {})",
                end_height.unwrap_or_default(),
                end_time.unwrap_or_default()
            ),
            ContractError::NotExpired => write!(f, "NotExpired"),
        }
    }
}
