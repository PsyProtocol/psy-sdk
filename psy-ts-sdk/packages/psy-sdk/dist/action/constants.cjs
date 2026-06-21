'use strict';

const PSY_NETWORK_MAGIC_MAINNET = 0x1337cf514544c069n;
const PSY_NETWORK_MAGIC_TESTNET = 0x1337cf514544c169n;
const PSY_NETWORK_MAGIC_REGTEST = 0x1337cf514544cf69n;
function getPsyNetworkMagicForNetworkId(networkId) {
    if (networkId === "mainnet") {
        return PSY_NETWORK_MAGIC_MAINNET;
    }
    else if (networkId === "testnet") {
        return PSY_NETWORK_MAGIC_TESTNET;
    }
    else if (networkId === "regtest") {
        return PSY_NETWORK_MAGIC_REGTEST;
    }
    else {
        throw new Error("Invalid networkId: '" + networkId + "'");
    }
}

exports.PSY_NETWORK_MAGIC_MAINNET = PSY_NETWORK_MAGIC_MAINNET;
exports.PSY_NETWORK_MAGIC_REGTEST = PSY_NETWORK_MAGIC_REGTEST;
exports.PSY_NETWORK_MAGIC_TESTNET = PSY_NETWORK_MAGIC_TESTNET;
exports.getPsyNetworkMagicForNetworkId = getPsyNetworkMagicForNetworkId;
