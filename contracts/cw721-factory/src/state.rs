use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Storage, Uint64};
use cosmwasm_storage::{singleton, singleton_read, ReadonlySingleton, Singleton};
use cw_storage_plus::{Map, Item};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub nft_code_id: Uint64,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct NftData {
    pub name: String,
    pub symbol: String
}

// NFT_SEQ is nft id counter
pub const NFT_SEQ: Item<Uint64> = Item::new("nft_seq");

// NFTs keeps nft information indexed by id
pub const NFTS: Map<Uint64, NftData> = Map::new("nfts");

