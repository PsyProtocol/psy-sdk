/**
 * GENERATED preset — do not edit by hand.
 * Source: parth-generic-v1 client_prover/config.json networks.localhost
 *       + psy-contracts/deployments/localhost/deployed-contracts.json
 *       (generatedAt: 2026-07-08T17:05:04.534Z)
 * Regenerate: node scripts/gen-presets.mjs [/path/to/parth-generic-v1]
 * Live consumers can always pass fresher values via createPsyWallet overrides.
 */
import type { PsyNetworkDefinition } from '../types'

export const localhost: PsyNetworkDefinition = {
  "name": "localhost",
  "psy": {
    "magic": "0x1337CF514544CF69",
    "users_per_realm": 1048576,
    "global_user_tree_height": 32,
    "realm_user_tree_height": 20,
    "group_realm_height": 1,
    "realm_configs": [
      {
        "id": 0,
        "rpc_url": [
          "http://127.0.0.1:13380"
        ]
      },
      {
        "id": 1,
        "rpc_url": [
          "http://127.0.0.1:13390"
        ]
      }
    ],
    "coordinator_configs": [
      {
        "id": 0,
        "rpc_url": [
          "http://127.0.0.1:1337"
        ]
      }
    ],
    "prove_proxy_url": [
      "http://127.0.0.1:9999"
    ],
    "api_services_url": [
      "http://127.0.0.1:3000"
    ],
    "indexer_graphql_url": [
      "http://127.0.0.1:8080/v1/graphql"
    ],
    "explorer_url": [
      "http://127.0.0.1:5178"
    ],
    "nostr_relay_urls": [
      "ws://127.0.0.1:8081"
    ],
    "whitelist": {
      "enabled": false,
      "secp256k1": []
    },
    "native_currency": "0",
    "native_currency_decimal": 9,
    "native_currency_name": "Psy",
    "native_currency_symbol": "PSY",
    "fees": {
      "register_user_fee": 0,
      "deploy_contract_fee": 0,
      "guta_fee": 1000000000,
      "da_fee": 1000
    },
    "l1_rpc_urls": [
      "http://127.0.0.1:8545"
    ]
  },
  "l1": {
    "chainId": 31337,
    "chain": {
      "chainId": 31337,
      "name": "Localhost",
      "shortName": "localhost",
      "psyIndex": 0,
      "nativeCurrency": {
        "name": "Ether",
        "symbol": "ETH",
        "decimals": 18
      },
      "explorerUrl": "",
      "rpcUrls": [
        "http://127.0.0.1:8545"
      ],
      "routerAddress": "0x59b670e9fA9D0A427751Af201D676719a970857b",
      "bridgeAddress": "0xA51c1fc2f0D1a1b8494Ed1FE312d7C3a78Ed91C0",
      "stateManagerAddress": "0x610178dA211FEF7D417bC0e6FeD39F05609AD788",
      "erc20GatewayAddress": "0x9A676e781A523b5d0C0e43731313A708CB607508",
      "wethAddress": "0x0B306BF915C4d645ff596e518fAf3F9669b97016",
      "stateManager": "0x610178dA211FEF7D417bC0e6FeD39F05609AD788",
      "erc20Gateway": "0x9A676e781A523b5d0C0e43731313A708CB607508",
      "mockUSDT": "0x3Aa5ebB10DC797CAC828524e59A333d0A371443c",
      "psyToken": "0x68B1D87F95878fE05B998F19b66F4baba5De1aed",
      "deployed": true
    }
  }
} as unknown as PsyNetworkDefinition
