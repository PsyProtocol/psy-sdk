import { Felt } from "../core";
import { ContractCallArgs } from "../local-prover-rpc";
import { IPsyTransactionSigner, IPsyTransactionSignerProvider } from "../zksigner";
import { NetworkId } from "../action";
interface ICorePsyUserInfo {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    userId: Felt;
    publicKeyHex: string;
}
interface IPsyCompleteUserInfo extends ICorePsyUserInfo {
    nonce: string;
    balance: Felt;
}
interface IPsyUserWallet {
    status: boolean;
    signer: IPsyTransactionSigner;
    getUserInfo(): Promise<IPsyCompleteUserInfo>;
    getBalance(): Promise<bigint>;
    getBalanceString(): Promise<string>;
    execContractCall(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string>;
}
interface IPsyUserWalletProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signerProvider: IPsyTransactionSignerProvider;
    getUserWallets(): Promise<IPsyUserWallet[]>;
}
export type { ICorePsyUserInfo, IPsyUserWallet, IPsyCompleteUserInfo, IPsyUserWalletProvider };
//# sourceMappingURL=types.d.ts.map