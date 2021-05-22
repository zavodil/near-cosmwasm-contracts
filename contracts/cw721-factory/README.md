# cw721-factory

This contract is for demonstrating sub messages functionality.
On instantiate, cw721 code id is set, and on `ExecuteMsg::CreateNft` contract instantiates
an NFT instance.
There are two options to demo reply, in `ExecuteMsg::CreateNft` `reply` field sets behaviour of submessage.
One case is `Always` where response returned to calling contract on both Error and Success, and calling contract
operates based on the response.
Other one is `OnError` where response returned to on execution failure case.

In [responses](responses) folder there are examples of responses.
