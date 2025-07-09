import { Felt } from "../core";
import {
    ContractCallArgs,
} from "../local-prover-rpc";
import { IQedTransactionSigner, IQedTransactionSignerProvider } from "../zksigner";
import { NetworkId } from "../action";

interface ICoreQedUserInfo {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    userId: Felt;
    publicKeyHex: string;
}

interface IQedCompleteUserInfo extends ICoreQedUserInfo {
    nonce: string;
    balance: Felt;
}

interface IQedUserWallet {
    // prover: IQedUserProverProvider;
    status: boolean;
    signer: IQedTransactionSigner;
    getUserInfo(): Promise<IQedCompleteUserInfo>;
    getBalance(): Promise<bigint>;
    getBalanceString(): Promise<string>;
    // getRandomKeypair(): Promise<WalletKeyPair>;
    // registerUser(privateKey: PrivateKey): Promise<PublicKey>;
    // getZKPublicKey(): Promise<PublicKey>;
    // importPrivateKey(privateKey: PrivateKey): Promise<PublicKey>;
    // deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    // getDeployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract>;
    execContractCall(pk_hash: string, contractCallArgs: ContractCallArgs | ContractCallArgs[]): Promise<string>;
    // transfer(recipient: SCNumberLike, amount: SCNumberLike, nonce?: SCNumberLike): Promise<void>;
}

interface IQedUserWalletProvider {
    networkId: NetworkId;
    l2NetworkMagic: bigint;
    signerProvider: IQedTransactionSignerProvider;
    getUserWallets(): Promise<IQedUserWallet[]>;
}

export type { ICoreQedUserInfo, IQedUserWallet, IQedCompleteUserInfo, IQedUserWalletProvider };
