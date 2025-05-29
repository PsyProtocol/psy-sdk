import { DogeNetworkId } from "doge-sdk/dist/types";

// Mainnet Magic = 0x1337CF514544F069n = 1384803358401163369
const NETWORK_MAGIC_DOGE_MAINNET = "1384803358401163369";
// Testnet Magic = 0x1337CF514544F169n = 1384803358401163625
const NETWORK_MAGIC_DOGE_TESTNET = "1384803358401163625";
// Regtest Magic = 0x1337CF514544FF69n = 1384803358401167209
const NETWORK_MAGIC_DOGE_REGTEST = "1384803358401167209";

// Sig Actions
// 'CDEPOSIT' = 0x5449534F50454443n (little-endian) = 6073477172600063043
const SIG_ACTION_CLAIM_DEPOSIT_MAGIC = "6073477172600063043";

// 'WITHDRAW' = 0x5741524448544957n (little-endian) = 6287397008010660183n
const SIG_ACTION_WITHDRAW_MAGIC = "6287397008010660183";

// 'SENDDOGE' = 0x45474F44444E4553n (little-endian) =
const SIG_ACTION_TRANSFER_MAGIC = "4992045866585834835";

function getCityNetworkMagicForNetworkId(networkId: DogeNetworkId) {
    if (networkId === "doge") {
        return NETWORK_MAGIC_DOGE_MAINNET;
    } else if (networkId === "dogeTestnet") {
        return NETWORK_MAGIC_DOGE_TESTNET;
    } else if (networkId === "dogeRegtest") {
        return NETWORK_MAGIC_DOGE_REGTEST;
    } else {
        throw new Error("Invalid networkId: '" + networkId + "'");
    }
}

export {
    NETWORK_MAGIC_DOGE_MAINNET,
    NETWORK_MAGIC_DOGE_TESTNET,
    NETWORK_MAGIC_DOGE_REGTEST,
    SIG_ACTION_CLAIM_DEPOSIT_MAGIC,
    SIG_ACTION_WITHDRAW_MAGIC,
    SIG_ACTION_TRANSFER_MAGIC,
    getCityNetworkMagicForNetworkId,
};
