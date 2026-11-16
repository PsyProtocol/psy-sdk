import { describe, expect, it, jest } from "@jest/globals";

import { IPsyUserProverProvider, SignType } from "../../local-prover-rpc";
import { NetworkId } from "../../action";
import { PsyMemoryTransactionSignerProvider } from "./provider";

// Canonical node-side measured fingerprints (psy_prover::wallet::memory_wallet)
// in QHashOut string form — limbs reversed, each as 16-hex big-endian — for
// stage magic 0x1337CF514544CF69 (localhost/testnet); measured node-side via
// psy_user_cli get-user-end-cap-common-data.
const ETH_PERSONAL_FINGERPRINT = "d02a6784ac56e446b2109dff922ce3d49e785be25dac781bb3b1d7770f871513";
const SECP256K1_FINGERPRINT = "64a459036da31084f2b043e036679592aa85bd9dc8df226fc4519c7e7f1ebf4a";
const ZK_FINGERPRINT = "2257f7b847ff2a04ad10b596baa1ac3209041a4e15d040de7abcb62fb4d5c89d";

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
