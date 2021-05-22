use cosmwasm_std::{
    attr, entry_point, to_binary, Addr, Binary, ContractResult, CosmosMsg, Deps, DepsMut, Env,
    Event, MessageInfo, Order, Reply, ReplyOn, Response, StdResult, SubMsg, SubcallResponse,
    Uint64, WasmMsg,
};

use crate::error::ContractError;
use crate::msg::{AllNftsResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, NftData, CONFIG, NFTS, NFT_SEQ};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    msg.validate()?;

    let config = Config {
        nft_code_id: msg.nft_code_id,
    };
    CONFIG.save(deps.storage, &config)?;

    NFT_SEQ.save(deps.storage, &Uint64::zero())?;

    Ok(Response::default())
}

// And declare a custom Error variant for the ones where you will want to make use of it
#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateNft {
            name,
            symbol,
            reply,
        } => execute_create_nft(deps, env, info, name, symbol, reply),
    }
}

// this code demonstrates reply always case
pub fn execute_create_nft(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    name: String,
    symbol: String,
    reply: ReplyOn,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let nft_id = NFT_SEQ.load(deps.storage)?;
    let nft_data = NftData {
        contract_addr: Addr::unchecked("unchecked"),
        name: name.clone(),
        symbol: symbol.clone(),
    };
    NFTS.save(deps.storage, nft_id.u64().into(), &nft_data)?;

    let instantiate_nft_msg = cw721_base::msg::InstantiateMsg {
        name,
        symbol,
        minter: env.contract.address.to_string(),
    };
    let msg = WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id: config.nft_code_id.u64(),
        msg: to_binary(&instantiate_nft_msg)?,
        send: vec![],
        label: "Created by NFT Factory".to_string(),
    };
    let cosmos_msg = CosmosMsg::Wasm(msg);

    let sub_msg = SubMsg {
        id: nft_id.u64(),
        msg: cosmos_msg,
        gas_limit: None,
        reply_on: reply,
    };
    let res = Response {
        submessages: vec![sub_msg],
        messages: vec![],
        attributes: vec![attr("action", "create_nft"), attr("nft_id", nft_id)],
        data: None,
    };
    Ok(res)
}

// This just stores the result for future query
#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    reply_create_nft(deps, env, msg.id.into(), msg.result)
}

pub fn reply_create_nft(
    deps: DepsMut,
    _env: Env,
    nft_id: Uint64,
    res: ContractResult<SubcallResponse>,
) -> Result<Response, ContractError> {
    match res {
        // TX will revert if not found, no need to remove nft data
        ContractResult::Err(_) => Err(ContractError::CreateFailed {}),
        ContractResult::Ok(subcall) => {
            let contract_addr = parse_contract_from_event(subcall.events)?;
            NFTS.update(deps.storage, nft_id.u64().into(), |exists| match exists {
                // NFT not found, this will not happen but good to cover
                None => Err(ContractError::NFTNotExists { nft_id }),
                // TX will finalize and nft data will be committed to the storage
                Some(mut nft) => {
                    nft.contract_addr = contract_addr.clone();
                    Ok(nft)
                }
            })
            .map(|_| Response {
                submessages: vec![],
                messages: vec![],
                attributes: vec![
                    attr("action", "reply_create_nft"),
                    attr("nft_id", nft_id),
                    attr("created_contract_addr", contract_addr),
                ],
                data: None,
            })
        }
    }
}

fn parse_contract_from_event(events: Vec<Event>) -> Result<Addr, ContractError> {
    events
        .into_iter()
        .find(|e| e.kind == "message")
        .and_then(|ev| {
            ev.attributes
                .into_iter()
                .find(|a| a.key == "contract_address")
        })
        .ok_or(ContractError::ParseError {})
        .map(|a| Addr::unchecked(a.value))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetNft { id } => {
            let nft = NFTS.load(deps.storage, id.u64().into())?;
            to_binary(&nft)
        }
        QueryMsg::GetAllNft {} => {
            let all_nfts = NFTS
                .range(deps.storage, None, None, Order::Ascending)
                .flat_map(|nft| nft.map(|d| d.1))
                .collect();
            let res = AllNftsResponse { all_nfts };
            to_binary(&res)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instantiate_works() {}
}
