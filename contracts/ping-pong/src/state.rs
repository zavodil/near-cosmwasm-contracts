use cosmwasm_std::Uint64;
use cw_storage_plus::Item;

/*
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {}

*/

pub const PING_COUNT: Item<Uint64> = Item::new("ping_count");
