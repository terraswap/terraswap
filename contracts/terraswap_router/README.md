# Terraswap Router <!-- omit in toc -->

The Router Contract contains the logic to facilitate multi-hop swap operations via terraswap.

**Only Terraswap is supported.**

phoenix-1 Contract:
- 

pisco-1 Contract: 
- https://finder.terra.money/testnet/address/terra1xp6xe6uwqrspumrkazdg90876ns4h78yw03vfxghhcy03yexcrcsdaqvc8

Tx: 
- Luna => DELIGHT => TNT: https://finder.terra.money/testnet/tx/CCBE3E2C746967A03CAD13B7FCAB4BD823BE54883290F3BEE7A213DC6096A39A

### Operations Assertion
The contract will check whether the resulting token is swapped into one token.

### Example

Swap Luna => DELIGHT => TNT
```
{
   "execute_swap_operations":{
      "operations":[
         {
            "terra_swap":{
               "offer_asset_info":{
                  "native_token":{
                     "denom":"uluna"
                  }
               },
               "ask_asset_info":{
                  "token":{
                     "contract_addr":"terra1cl0kw9axzpzkw58snj6cy0hfp0xp8xh9tudpw2exvzuupn3fafwqqhjc24"
                  }
               }
            }
         },
         {
            "terra_swap":{
               "offer_asset_info":{
                  "token":{
                     "contract_addr":"terra1cl0kw9axzpzkw58snj6cy0hfp0xp8xh9tudpw2exvzuupn3fafwqqhjc24"
                  }
               },
               "ask_asset_info":{
                  "token":{
                     "contract_addr":"terra1qnypzwqa03h8vqs0sxjp8hxw0xy5zfwyax26jgnl5k4lw92tjw0scdkrzm"
                  }
               }
            }
         }
      ],
      "minimum_receive":"1"
   }
}
```