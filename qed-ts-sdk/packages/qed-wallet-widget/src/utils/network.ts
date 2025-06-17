import { NetworkId } from "@qed/qed-sdk/src/action";

function getNetworkNameById(id: NetworkId) {
    if (id === "mainnet") {
        return "Qed mainnet";
    } else if (id === "testnet") {
        return "Qed Testnet";
    } else if (id === "regtest") {
        return "Qed Regtest";
    } else {
        return "Unknown";
    }
}

export { getNetworkNameById };
