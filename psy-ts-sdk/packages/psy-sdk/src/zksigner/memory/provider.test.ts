import { describe, expect, it, test, jest } from "@jest/globals";

import { IPsyUserProverProvider, SignType } from "../../local-prover-rpc";
import { NetworkId, getPsyNetworkMagicForNetworkId } from "../../action";
import { PsyMemoryTransactionSignerProvider } from "./provider";

import { signatureFingerprints } from "./signature-fingerprints";

const ETH_PERSONAL_FINGERPRINT = signatureFingerprints["eth-personal-secp256k1"];
const SECP256K1_FINGERPRINT = signatureFingerprints.secp256k1;
const ZK_FINGERPRINT = signatureFingerprints.zk;

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

    it("selects the compiled classic secp256k1 held-key fingerprint", async () => {
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

describe('compiled built-in signature fingerprints', () => {
    for (const signType of [SignType.ZKSign, SignType.SECP256K1Sign, SignType.EthPersonalSECP256K1Sign] as const) {
        test(`${signType} selects the compiled fingerprint and rejects a mismatch before import`, async () => {
            const addUser = jest.fn(async (_privateKey: string, _signType: SignType, _fingerprint: string) => 'a'.repeat(64));
            const provider = new PsyMemoryTransactionSignerProvider({ addUser } as unknown as IPsyUserProverProvider, 'testnet');
            const fingerprint = signatureFingerprints[signType];
            const signer = await provider.importPrivateKey('1'.repeat(64), signType, fingerprint);
            expect(await signer.getFingerprint!()).toEqual(fingerprint);
            expect(addUser).toHaveBeenCalledWith('1'.repeat(64), signType, fingerprint);
            await expect(provider.importPrivateKey('2'.repeat(64), signType, '0'.repeat(64))).rejects.toThrow('fingerprint mismatch');
            expect(addUser).toHaveBeenCalledTimes(1);
        });
    }
    test('testnet actions use canonical CF69 rather than the obsolete C169 magic', () => {
        expect(getPsyNetworkMagicForNetworkId('testnet')).toEqual(0x1337cf514544cf69n);
        expect(getPsyNetworkMagicForNetworkId('testnet')).not.toEqual(0x1337cf514544c169n);
    });
});
