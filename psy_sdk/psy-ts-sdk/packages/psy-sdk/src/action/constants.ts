type NetworkId = "mainnet" | "testnet" | "regtest";

const PSY_NETWORK_MAGIC_MAINNET = 0x1337cf514544c069n;
const PSY_NETWORK_MAGIC_TESTNET = 0x1337cf514544c169n;
const PSY_NETWORK_MAGIC_REGTEST = 0x1337cf514544cf69n;

// Sig Actions
// 'CDEPOSIT' = 0x5449534F50454443n (little-endian) = 6073477172600063043
const SIG_ACTION_CLAIM_DEPOSIT_MAGIC = "6073477172600063043";

// 'WITHDRAW' = 0x5741524448544957n (little-endian) = 6287397008010660183n
const SIG_ACTION_WITHDRAW_MAGIC = "6287397008010660183";

// 'SENDDOGE' = 0x45474F44444E4553n (little-endian) =
const SIG_ACTION_TRANSFER_MAGIC = "4992045866585834835";

function getPsyNetworkMagicForNetworkId(networkId: NetworkId) {
    if (networkId === "mainnet") {
        return PSY_NETWORK_MAGIC_MAINNET;
    } else if (networkId === "testnet") {
        return PSY_NETWORK_MAGIC_TESTNET;
    } else if (networkId === "regtest") {
        return PSY_NETWORK_MAGIC_REGTEST;
    } else {
        throw new Error("Invalid networkId: '" + networkId + "'");
    }
}

export {
    NetworkId,
    PSY_NETWORK_MAGIC_MAINNET,
    PSY_NETWORK_MAGIC_TESTNET,
    PSY_NETWORK_MAGIC_REGTEST,
    SIG_ACTION_CLAIM_DEPOSIT_MAGIC,
    SIG_ACTION_WITHDRAW_MAGIC,
    SIG_ACTION_TRANSFER_MAGIC,
    getPsyNetworkMagicForNetworkId,
};
