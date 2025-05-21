import { Hash256, HexString } from "../rpc/baseTypes";

enum CityUserProverRPCCommand {
    ProveSecp256K1Signature = "cr_prove_secp256k1_signature",
    ProveZKSignature = "cr_prove_zk_signature",
    ProveZKSignatureEnc = "cr_prove_zk_signature_enc",
    GetZKPublicKey = "cr_get_zk_public_key",
    GetZKPublicKeyEnc = "cr_get_zk_public_key_enc",
    GetResult = "cr_get_result",
}

interface ICityZKSignatureProver {
    zkSignHash(privateKey: Hash256, message: Hash256): Promise<HexString>;
    getZKPublicKeyForPrivateKey(privateKey: Hash256): Promise<HexString>;
}
interface ICitySecp256K1SignatureProver {
    generateSecp256K1SignatureProof(publicKey: HexString, signature: HexString, message: Hash256): Promise<HexString>;
}
interface ICityWalletProver extends ICityZKSignatureProver, ICitySecp256K1SignatureProver {}
interface ICityUserProverProvider extends ICityWalletProver {
    proveSecp256K1SignatureBase(publicKey: HexString, signature: HexString, message: Hash256): Promise<Hash256>;
    proveZKSignatureBase(privateKey: Hash256, message: Hash256): Promise<Hash256>;
    proveZKSignatureEncBase(encryptedPrivateKey: Hash256, message: Hash256, salt: Hash256): Promise<Hash256>;
    getZKPublicKeyBase(privateKey: Hash256): Promise<Hash256>;
    getZKPublicKeyEncBase(encryptedPrivateKey: Hash256, salt: Hash256): Promise<Hash256>;

    proveSecp256K1Signature(
        publicKey: HexString,
        signature: HexString,
        message: Hash256,
        maxAttempts?: number,
        delay?: number
    ): Promise<HexString>;
    proveZKSignature(privateKey: Hash256, message: Hash256, maxAttempts?: number, delay?: number): Promise<HexString>;
    proveZKSignatureEnc(
        encryptedPrivateKey: Hash256,
        message: Hash256,
        salt: Hash256,
        maxAttempts?: number,
        delay?: number
    ): Promise<HexString>;
    getZKPublicKey(privateKey: Hash256, maxAttempts?: number, delay?: number): Promise<HexString>;
    getZKPublicKeyEnc(
        encryptedPrivateKey: Hash256,
        salt: Hash256,
        maxAttempts?: number,
        delay?: number
    ): Promise<HexString>;

    getResult(hash: Hash256): Promise<HexString>;
}

export type { ICityUserProverProvider, ICityZKSignatureProver, ICitySecp256K1SignatureProver, ICityWalletProver };

export { CityUserProverRPCCommand };
