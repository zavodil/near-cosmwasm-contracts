use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint64};
use cw_storage_plus::{Item, Map, U64Key};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub nft_code_id: Uint64,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub struct NftData {
    pub contract_addr: Addr,
    pub name: String,
    pub symbol: String,
}

// NFT_SEQ is nft id counter
pub const NFT_SEQ: Item<Uint64> = Item::new("nft_seq");

// NFTs keeps nft information indexed by id
pub const NFTS: Map<U64Key, NftData> = Map::new("nfts");
