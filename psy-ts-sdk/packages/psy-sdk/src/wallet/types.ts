import { Felt } from "../core";
import {
    ContractCallArgs,
} from "../local-prover-rpc";
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
    // prover: IPsyUserProverProvider;
    status: boolean;
    signer: IPsyTransactionSigner;
    getUserInfo(): Promise<IPsyCompleteUserInfo>;
    getBalance(): Promise<bigint>;
    getBalanceString(): Promise<string>;
    // getRandomKeypair(): Promise<WalletKeyPair>;
    // registerUser(privateKey: PrivateKey): Promise<PublicKey>;
    // getZKPublicKey(): Promise<PublicKey>;
    // importPrivateKey(privateKey: PrivateKey): Promise<PublicKey>;
    // deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    // getDeployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract>;
    execContractCall(user_id: Felt, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string>;
    // transfer(recipient: SCNumberLike, amount: SCNumberLike, nonce?: SCNumberLike): Promise<void>;
}

interface IPsyUserWalletProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signerProvider: IPsyTransactionSignerProvider;
    getUserWallets(): Promise<IPsyUserWallet[]>;
}

export type { ICorePsyUserInfo, IPsyUserWallet, IPsyCompleteUserInfo, IPsyUserWalletProvider };
