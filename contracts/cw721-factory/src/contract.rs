use cosmwasm_std::{entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Reply, ReplyOn, StdError, ContractResult, SubcallResponse, Addr, Uint64, CosmosMsg, WasmMsg};

use crate::error::ContractError;
use crate::msg::{CountResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{config, config_read, Config, NFT_SEQ, CONFIG, NFTS, NftData};

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

    let config = Config { nft_code_id: msg.nft_code_id };
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
        ExecuteMsg::CreateNft { name, symbol } => execute_create_nft(deps, env, info, name, symbol),
    }
}

pub fn execute_create_nft(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    name: String,
    symbol: String
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let nft_id = NFT_SEQ.load(deps.storage)?;
    let nft_data = NftData{ name, symbol };
    NFTS.save(deps.storage, nft_id.key(), &nft_data)?;

    let instantiate_nft_msg = cw721_base::msg::InstantiateMsg {
        name: name.clone(),
        symbol: symbol.clone(),
        minter: env.contract.address.to_string()
    };
    let msg = WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id: config.nft_code_id.u64(),
        msg: to_binary(&instantiate_nft_msg)?,
        send: vec![],
        label: "Created by NFT Factory".to_string()
    };
    let cosmos_msg = CosmosMsg::Wasm(msg);
    let mut res = Response::new();
    res.add_submessage(nft_id.u64(), vec![cosmos_msg], None, ReplyOn::Error);

    Ok(res)
}

// This just stores the result for future query
#[entry_point]
pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
    reply_create_nft(deps, env, msg.id.into(), msg.result)
}

pub fn reply_create_nft(
    deps: DepsMut,
    env: Env,
    nft_id: Uint64,
    res: ContractResult<SubcallResponse>
) -> Result<Response, ContractError> {
    match res {
        // Remove nft data if there is an error
        ContractResult::Err(err) => match err.as_str() {
            _ => {
                NFTS.remove(deps.storage, nft_id.key())?;
                Ok()
            },
        },
        ContractResult::Ok(subcall) => {
            // extract contract_addr from events
            let contract_addr = subcall.events.into_iter()
                .find(|e| e.kind == "wasm")
                .and_then(|e| {
                    e.attributes.into_iter()
                        .find(|a| a.key == "contract_address")
                        .map(|a| Addr::unchecked(a.value))
                }).ok_or_else(Err(ContractError::Unauthorized {}))?;
            // save contract_addr

        }
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn instantiate_works() {
    }

}
