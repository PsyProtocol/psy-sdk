import { ZKPublicKeyInfo } from "./ZKPublicKeyInfo";
import { calculatePkHash } from "./pkhash";


describe("calculatePkHash", () => {
    it("should calculate the correct pk hash", () => {
        const zkPublicKeyInfo: ZKPublicKeyInfo = {
            fingerprint: "d2f572f1402fa8a92c9af0a2226e05ef8f5f4f34d764c6515b90d2b391fc48c1",
            public_key_param: "7cdc8b38073d176578f62fcbf9432622272451f773edd472b12a09d81d5b2a91",
        };
        const pkHash = calculatePkHash(zkPublicKeyInfo);
        expect(pkHash).toBe("0x56f5ba3790fce2de559aac368adb33d9418ba240ee4c6aec83881bbad50f8a29");
    });
});