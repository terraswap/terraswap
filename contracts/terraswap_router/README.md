# Terraswap Router <!-- omit in toc -->

The Router Contract contains the logic to facilitate multi-hop swap operations via terraswap.

**Only Terraswap is supported.**

phoenix-1 Contract:
- 

pisco-1 Contract: 
- https://finder.terra.money/testnet/address/terra1mgrfjp339t4xg4zger3643v88k7p3mppsyj6vny0ua20lx24rpkswxd44a

Tx: 
- Luna => DELIGHT => DELTEST: https://finder.terra.money/testnet/tx/141F4411A78352C173D27D05A961421C0E0276FB81E232DD8CE20F053AE3B52A

### Operations Assertion
The contract will check whether the resulting token is swapped into one token.

### Example

Swap Luna => DELIGHT => DELTEST
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
                     "contract_addr":"terra1scqz2m7rx87w8m0g9jtj5jyrudduuqyykaecfu5lrd95dy0dt3wscsk4jn"
                  }
               }
            }
         },
         {
            "terra_swap":{
               "offer_asset_info":{
                  "token":{
                     "contract_addr":"terra1scqz2m7rx87w8m0g9jtj5jyrudduuqyykaecfu5lrd95dy0dt3wscsk4jn"
                  }
               },
               "ask_asset_info":{
                  "token":{
                     "contract_addr":"terra1pl5xjwmn2wldyntyrpcc0k944esxhw8jlj49dywrt0fqphuypgds699xuu"
                  }
               }
            }
         }
      ],
      "minimum_receive":"1"
   }
}
```