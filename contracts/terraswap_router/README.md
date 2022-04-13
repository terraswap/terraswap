# Terraswap Router <!-- omit in toc -->

The Router Contract contains the logic to facilitate multi-hop swap operations via native & terraswap.

**On-chain swap & Terraswap is supported.**

Columbus-5 Contract:
- https://finder.terra.money/mainnet/address/terra19f36nz49pt0a4elfkd6x7gxxfmn3apj7emnenf

Bombay-0012 Contract: 
- https://finder.terra.money/testnet/address/terra1c58wrdkyc0ynvvxcv834kz65nfsxmw2w0pwusq

Tx: 
- KRT => UST => mABNB: https://finder.terra.money/testnet/tx/46A1C956D2F4F7A1FA22A8F93749AEADB953ACDFC1B9FB7661EEAB5C59188175
- mABNB => UST => KRT:  https://finder.terra.money/testnet/tx/e9d63ce2c8ac38f6c9434c62f9a8b59f38259feb86f075d43c253ea485d7f0a9

### Operations Assertion
The contract will check whether the resulting token is swapped into one token.

### Example

Swap KRT => UST => mABNB
```
{
   "execute_swap_operations":{
      "operations":[
         {
            "native_swap":{
               "offer_denom":"ukrw",
               "ask_denom":"uusd"
            }
         },
         {
            "terra_swap":{
               "offer_asset_info":{
                  "native_token":{
                     "denom":"uusd"
                  }
               },
               "ask_asset_info":{
                  "token":{
                     "contract_addr":"terra1avryzxnsn2denq7p2d7ukm6nkck9s0rz2llgnc"
                  }
               }
            }
         }
      ],
      "minimum_receive":"88000"
   }
}
```
