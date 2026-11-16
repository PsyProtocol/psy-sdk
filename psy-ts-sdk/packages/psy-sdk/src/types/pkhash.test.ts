import { ZKPublicKeyInfo } from "./ZKPublicKeyInfo";
import { calculatePkHash } from "./pkhash";


describe("calculatePkHash", () => {
    // Keep each fingerprint paired with its own Poseidon hash; changing only
    // the input previously left the June 2025 expected hash on a newer key.
    it.each([
        [
            "original June 2025 vector",
            "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0",
            "0x56f5ba3790fce2de559aac368adb33d9418ba240ee4c6aec83881bbad50f8a29",
        ],
        [
            "January 2026 fingerprint",
            "2257f7b847ff2a04ad10b596baa1ac3209041a4e15d040de7abcb62fb4d5c89d",
            "0x5297625c1b053415167b9b32448181b9d4163e49d61132e7e6ced4a26b252c02",
        ],
    ])("hashes the %s", (_name, fingerprint, expected) => {
        const zkPublicKeyInfo: ZKPublicKeyInfo = {
            fingerprint,
            public_key_param: "7cdc8b38073d176578f62fcbf9432622272451f773edd472b12a09d81d5b2a91",
        };
        const pkHash = calculatePkHash(zkPublicKeyInfo);
        expect(pkHash).toBe(expected);
    });
});
