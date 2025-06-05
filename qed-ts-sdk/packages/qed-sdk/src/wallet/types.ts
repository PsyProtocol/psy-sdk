import { PrivateKey, PublicKey, QHashOut } from "../core";
import {
    ContractCallArgs,
    DPNFunctionCircuitDefinition,
    IQEDUserProverProvider,
    WalletKeyPair,
} from "../local-prover-rpc";
import { ZKPublicKeyInfo } from "../types";
interface ICoreQedUserInfo {
    userId: number;
    publicKeyHex: string;
}

interface IQedUserWallet {
    prover: IQEDUserProverProvider;
    getRandomKeypair(): Promise<WalletKeyPair>;
    registerUser(privateKey: PrivateKey): Promise<QHashOut>;
    getZKPublicKey(): Promise<ZKPublicKeyInfo>;
    importPrivateKey(privateKey: PrivateKey): Promise<PublicKey>;
    deployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string>;
    contractCall(contractCallArgs: ContractCallArgs[]): Promise<string>;
}

export type { ICoreQedUserInfo, IQedUserWallet };
