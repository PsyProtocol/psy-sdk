import { describe, expect, it } from "@jest/globals";

import { SignType } from "./types";

describe("SignType", () => {
    it("exposes eth-personal-secp256k1 as the EIP-191 personal_sign variant", () => {
        expect(SignType.EthPersonalSECP256K1Sign).toBe("eth-personal-secp256k1");
    });

    it("leaves the classic sign type string values unchanged", () => {
        expect(SignType.ZKSign).toBe("zk");
        expect(SignType.SECP256K1Sign).toBe("secp256k1");
        expect(SignType.SoftwareDefinedDPNSign).toBe("software-defined-dpn");
        expect(SignType.SoftwareDefinedPlonky2Sign).toBe("software-defined-plonky2");
        expect(SignType.SDKeySign).toBe("sd-key");
    });
});
