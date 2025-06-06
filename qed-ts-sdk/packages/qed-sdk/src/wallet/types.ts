import { Felt, PrivateKey, PublicKey, SCNumberLike } from "../core";
import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IQEDUserProverProvider,
    WalletKeyPair,
} from "../local-prover-rpc";

interface ICoreQedUserInfo {
    userId: Felt;
    publicKeyHex: string;
}

interface IQedCompleteUserInfo extends ICoreQedUserInfo {
    nonce: string;
    balance: Felt;
}

interface IQedUserWallet {
    prover: IQEDUserProverProvider;
    getUserInfo(): Promise<IQedCompleteUserInfo>;
    getBalance(): Promise<bigint>;
    getBalanceString(): Promise<string>;
    getRandomKeypair(): Promise<WalletKeyPair>;
    registerUser(privateKey: PrivateKey): Promise<PublicKey>;
    getZKPublicKey(): Promise<PublicKey>;
    importPrivateKey(privateKey: PrivateKey): Promise<PublicKey>;
    deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    contractCall(contractCallArgs: ContractCallArgs[]): Promise<string>;
    transfer(recipient: SCNumberLike, amount: SCNumberLike, nonce?: SCNumberLike): Promise<void>;
}

export type { ICoreQedUserInfo, IQedUserWallet, IQedCompleteUserInfo };
