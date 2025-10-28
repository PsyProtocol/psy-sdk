import { NetworkId } from "@qed/psy-sdk/src/action";

function getNetworkNameById(id: NetworkId) {
    if (id === "mainnet") {
        return "Psy Mainnet";
    } else if (id === "testnet") {
        return "Psy Testnet";
    } else if (id === "regtest") {
        return "Psy Regtest";
    } else {
        return "Unknown";
    }
}

export { getNetworkNameById };
