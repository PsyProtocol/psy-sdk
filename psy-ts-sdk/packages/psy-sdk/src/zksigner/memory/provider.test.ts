import { describe, expect, it, jest } from "@jest/globals";

import { IPsyUserProverProvider, SignType } from "../../local-prover-rpc";
import { NetworkId } from "../../action";
import { PsyMemoryTransactionSignerProvider } from "./provider";

// Canonical get_eth_personal_secp256k1_fingerprint() — little-endian-limb hex of
// ETH_PERSONAL_SECP256K1_FINGERPRINT_U64, mirroring the existing hardcoded
// fingerprint convention the held-key memory provider uses for ZK / secp256k1.
const ETH_PERSONAL_FINGERPRINT = "4cf514982eb7155648bf1b7852a6a564d8e86998cc1c6365a50e15796b7f0745";
const SECP256K1_FINGERPRINT = "320d034234f0dab4d02c4b03d69276cbd5c2eb831aca1b11c7e52078ace2e33b";
const ZK_FINGERPRINT = "65e0169bfffd55f1c0ea9f76c111a5b15e652322ee253c1a9604a10d59066b50";

const networkId: NetworkId = "testnet";

/**
 * Recording prover stub: only `addUser` is exercised by the held-key path
 * (PsyMemoryTransactionSigner.create -> proverProvider.addUser). Capturing its
 * positional arguments lets us assert the exact (privateKey, signType,
 * fingerprint) triple the memory provider forwards, without touching WASM.
 */
function recordingProvider(): { provider: IPsyUserProverProvider; addUser: jest.Mock } {
    const addUser = jest.fn(async () => "pk-hash-stub");
    const provider = { addUser } as unknown as IPsyUserProverProvider;
    return { provider, addUser };
}

describe("PsyMemoryTransactionSignerProvider held-key fingerprint mapping", () => {
    it("maps eth-personal-secp256k1 to the canonical eth_personal_secp256k1 fingerprint", async () => {
        const { provider, addUser } = recordingProvider();
        const signerProvider = new PsyMemoryTransactionSignerProvider(provider, networkId);

        await signerProvider.addRandomPrivateKey(SignType.EthPersonalSECP256K1Sign);

        expect(addUser).toHaveBeenCalledTimes(1);
        expect(addUser).toHaveBeenCalledWith(expect.any(String), SignType.EthPersonalSECP256K1Sign, ETH_PERSONAL_FINGERPRINT);
    });

    it("leaves the classic secp256k1 held-key fingerprint unchanged", async () => {
        const { provider, addUser } = recordingProvider();
        const signerProvider = new PsyMemoryTransactionSignerProvider(provider, networkId);

        await signerProvider.addRandomPrivateKey(SignType.SECP256K1Sign);

        expect(addUser).toHaveBeenCalledWith(expect.any(String), SignType.SECP256K1Sign, SECP256K1_FINGERPRINT);
    });

    it("leaves the classic zk held-key fingerprint unchanged", async () => {
        const { provider, addUser } = recordingProvider();
        const signerProvider = new PsyMemoryTransactionSignerProvider(provider, networkId);

        await signerProvider.addRandomPrivateKey(SignType.ZKSign);

        expect(addUser).toHaveBeenCalledWith(expect.any(String), SignType.ZKSign, ZK_FINGERPRINT);
    });
});
