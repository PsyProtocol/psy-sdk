import { describe, expect, it, jest } from "@jest/globals";

import { IPsyUserProverProvider, SignType } from "../../local-prover-rpc";
import { NetworkId } from "../../action";
import { PsyMemoryTransactionSignerProvider } from "./provider";

// Canonical node-side measured fingerprints (psy_prover::wallet::memory_wallet)
// in QHashOut string form — limbs reversed, each as 16-hex big-endian — for
// stage magic 0x1337CF514544CF69 (localhost/testnet). The eth-personal value is
// additionally verified against the rebuilt local-web-prover WASM: the
// add_user fingerprint dispatch accepts exactly this value.
const ETH_PERSONAL_FINGERPRINT = "ecebdc4f04de4300441dd9fa0704c905c7d2c4db90ceb612459a266481b88a6d";
const SECP256K1_FINGERPRINT = "b3fe2b5b448751fa150f312ed92e9e2d7ab14c1eff9d43dcab36b57ad44f06a3";
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
