import { NetworkId } from "../../action";
import { ContractCallArgs, IPsyUserProverProvider, SignType } from "../../local-prover-rpc";
import { IPsyTransactionSigner, IPsyTransactionSignerProvider, TPsyTransactionSignerProviderAbility } from "../types";
import { PsyMemoryTransactionSigner } from "./signer";
declare class PsyMemoryTransactionSignerProvider implements IPsyTransactionSignerProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signers: PsyMemoryTransactionSigner[];
    proverProvider: IPsyUserProverProvider;
    constructor(proverProvider: IPsyUserProverProvider, networkId: NetworkId);
    getSigners(): Promise<IPsyTransactionSigner[]>;
    getPublicKeysHex(): Promise<string[]>;
    getSignerByPublicKeyHex(publicKeyHex: string): Promise<IPsyTransactionSigner>;
    getAbilities(): TPsyTransactionSignerProviderAbility[];
    importPrivateKey(privateKeyHex: string, signType: SignType, fingerprint: string): Promise<IPsyTransactionSigner>;
    addRandomPrivateKey(signType: SignType): Promise<IPsyTransactionSigner>;
    private getFingerprintForSignType;
    registerUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    addUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string>;
    getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]>;
    claimRewards(pk_hash: string, jobInfos: string): Promise<string>;
}
export { PsyMemoryTransactionSignerProvider };
//# sourceMappingURL=provider.d.ts.map