/**
 * GENERATED preset — do not edit by hand.
 * Source: parth-generic-v1 client_prover/config.json networks.sepolia
 *       + psy-contracts/deployments/sepolia/deployed-contracts.json
 *       (generatedAt: 2026-06-02T08:13:52Z)
 * Regenerate: node scripts/gen-presets.mjs [/path/to/parth-generic-v1]
 * Live consumers can always pass fresher values via createPsyWallet overrides.
 */
import type { PsyNetworkDefinition } from '../types'

export const testnet: PsyNetworkDefinition = {
  "name": "testnet",
  "psy": {
    "magic": "0x1337CF514544C169",
    "users_per_realm": 1048576,
    "global_user_tree_height": 32,
    "realm_user_tree_height": 20,
    "group_realm_height": 1,
    "realm_configs": [
      {
        "id": 0,
        "rpc_url": [
          "https://realm0-stg.psy-protocol.xyz"
        ]
      },
      {
        "id": 1,
        "rpc_url": [
          "https://realm1-stg.psy-protocol.xyz"
        ]
      }
    ],
    "coordinator_configs": [
      {
        "id": 0,
        "rpc_url": [
          "https://coordinator-stg.psy-protocol.xyz"
        ]
      }
    ],
    "prove_proxy_url": [
      "http://127.0.0.1:9999"
    ],
    "api_services_url": [
      "https://services-stg.psy-protocol.xyz"
    ],
    "indexer_graphql_url": [
      "https://indexer-stg.psy-protocol.xyz/v1/graphql"
    ],
    "explorer_url": [
      "https://explorer-stg.psy-protocol.xyz"
    ],
    "nostr_relay_urls": [
      "wss://nostr-stg.psy-protocol.xyz/"
    ],
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
    "anvilForkSourceUrlEnv": "SEPOLIA_RPC_URL",
    "l1_rpc_urls": [
      "https://ethereum-sepolia-rpc.publicnode.com"
    ]
  },
  "l1": {
    "chainId": 11155111,
    "chain": {
      "chainId": 11155111,
      "name": "Sepolia",
      "shortName": "sepolia",
      "psyIndex": 0,
      "nativeCurrency": {
        "name": "Ether",
        "symbol": "ETH",
        "decimals": 18
      },
      "explorerUrl": "https://sepolia.etherscan.io",
      "rpcUrls": [
        "https://ethereum-sepolia-rpc.publicnode.com"
      ],
      "routerAddress": "0x598943197CE7C6c0be798429E061d181956F2dA5",
      "bridgeAddress": "0x9fE2145048272444bF186df1f7c704dde0397a5f",
      "stateManagerAddress": "0x27c972F97ba632F16ABb3b2C7e59f078189B839a",
      "erc20GatewayAddress": "0xb657619120d42a1FE1288f5F8726D95128dd9BFb",
      "wethAddress": "0xa52771c1C641CfEDdf1032C2502c696a18C030A4",
      "stateManager": "0x27c972F97ba632F16ABb3b2C7e59f078189B839a",
      "erc20Gateway": "0xb657619120d42a1FE1288f5F8726D95128dd9BFb",
      "mockUSDT": "0x4E15d3b2D4c28fBc2fc300136Eb62fB7Ae9D5De6",
      "psyToken": "0xD3c18ed548C540E0f5Eb3f6c6daF658C3E5A1d47",
      "deployed": true
    }
  }
} as unknown as PsyNetworkDefinition
