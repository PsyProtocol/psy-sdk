import { DogeNetworkId, hexToU8ArrayReversed, u8ArrayToHex } from "doge-sdk";
import {
    SCNumberLike,
    ICityTokenTransferRPCRequest,
    ICityUserState,
    ICityClaimDepositRPCRequest,
    ICityAddWithdrawalRPCRequest,
    ICityL1Deposit,
} from "../rpc/baseTypes";
import { ICityTransactionSigner } from "../zksigner/types";
import { ICityCompleteUserInfo, ICityUserWallet } from "./types";
import { ICityRPCProvider } from "../rpc/types";
import { getCityNetworkMagicForNetworkId } from "../action/constants";
import { DEPOSIT_FEE_AMOUNT, MAX_CHECKPOINT_ID, WITHDRAWAL_FEE_AMOUNT } from "../constants";
import { cityFelt, hashOutHex, reverseHexBytes } from "../utils/felt";
import { ICitySecp256K1SignatureProver } from "../userProverRPC/types";
import {
    computeSigActionHash,
    getClaimDepositSigAction,
    getTransferSigAction,
    getWithdrawalSigAction,
} from "../action/sighash";
import { ICitySigAction } from "../action/types";
import { getDecodedAddress } from "../utils/address";
import { userWalletCache } from "./cache";

class CityUserWallet implements ICityUserWallet {
    signer: ICityTransactionSigner;
    rpc: ICityRPCProvider;

    networkId: DogeNetworkId;
    l2NetworkMagic: string;
    userId: number;
    publicKeyHex: string;
    lastNonce: bigint = BigInt(0);
    localBalance: bigint = BigInt(0);
    unprocessedCheckpointId: number = -1;
    unprocessedOutflows: bigint = BigInt(0);
    unprocessedInflows: bigint = BigInt(0);

    constructor(signer: ICityTransactionSigner, rpc: ICityRPCProvider, userId: number, publicKeyHex: string) {
        this.signer = signer;
        this.rpc = rpc;
        this.networkId = rpc.getDogeNetworkId();
        this.userId = userId;
        this.l2NetworkMagic = getCityNetworkMagicForNetworkId(this.networkId);
        this.publicKeyHex = publicKeyHex;
    }
    static async create(signer: ICityTransactionSigner, rpc: ICityRPCProvider, userId: number) {
        const publicKeyHex = await signer.getPublicKeyHex();
        const wallet = new CityUserWallet(signer, rpc, userId, publicKeyHex);
        await wallet.refresh();
    }
    async refresh(): Promise<ICityUserState> {
        const { user, cache } = await userWalletCache.refreshUserFull(this.rpc, this.userId);

        user.balance = cache.localBalance;
        user.nonce = cache.localNonce;

        return user;
    }
    async getUserInfo(): Promise<ICityCompleteUserInfo> {
        const user = await this.refresh();

        return {
            networkId: this.networkId,
            l2NetworkMagic: this.l2NetworkMagic,
            userId: this.userId,
            publicKeyHex: this.publicKeyHex,
            nonce: user.nonce + "",
            balance: BigInt(user.balance + ""),
        };
    }
    async getBalance(): Promise<bigint> {
        const b = await this.refresh();
        return cityFelt(b.balance);
    }
    async getBalanceString(): Promise<string> {
        const balance = await this.getBalance();
        return balance.toString();
    }
    async getClaimDepositMessageHash(
        txidOrDepositId: string | number
    ): Promise<{ hash: string; deposit: ICityL1Deposit }> {
        const deposit = await (typeof txidOrDepositId === "number"
            ? this.rpc.getDepositById(MAX_CHECKPOINT_ID, 0)
            : this.rpc.getDepositByTxid(txidOrDepositId));
        const sigAction = getClaimDepositSigAction({
            network_magic: this.l2NetworkMagic,
            user: this.userId,
            transaction_id: u8ArrayToHex(hexToU8ArrayReversed(deposit.txid)),
            amount: cityFelt(deposit.value),
            deposit_fee: DEPOSIT_FEE_AMOUNT,
        });
        const hash = computeSigActionHash(sigAction);
        return { hash: hashOutHex(hash), deposit };
    }
    async zkSignSigAction(sigAction: ICitySigAction): Promise<string> {
        console.log("sigAction", sigAction);
        const abilities = this.signer.getAbilities();
        if (abilities.includes("sign-sigaction") && typeof this.signer.signSigAction === "function") {
            return this.signer.signSigAction(sigAction);
        } else if (abilities.includes("sign-hash") && typeof this.signer.signHash === "function") {
            return this.signer.signHash(hashOutHex(computeSigActionHash(sigAction)));
        } else {
            throw new Error("Signer does not support signing sig actions or hashes");
        }
    }
    async prepareTransfer(
        recipient: SCNumberLike,
        amount: SCNumberLike,
        nonce?: SCNumberLike | undefined
    ): Promise<ICityTokenTransferRPCRequest> {
        const realNonce =
            typeof nonce !== "undefined"
                ? nonce
                : await userWalletCache.processTransfer(this.rpc, this.userId, recipient, amount);
        const sigAction = getTransferSigAction({
            recipient: Number(recipient),
            amount: amount,
            nonce: realNonce,
            network_magic: this.l2NetworkMagic,
            user: this.userId,
        });
        const sig = await this.zkSignSigAction(sigAction);
        return {
            signature_proof: sig,
            user_id: this.userId,
            nonce: cityFelt(realNonce),
            value: cityFelt(amount),
            to: Number(recipient),
        };
    }
    async prepareWithdrawal(
        l1Address: string,
        amount: SCNumberLike,
        nonce?: SCNumberLike | undefined
    ): Promise<ICityAddWithdrawalRPCRequest> {
        const decoded = getDecodedAddress(l1Address);
        const realNonce =
            typeof nonce !== "undefined"
                ? nonce
                : await userWalletCache.processWithdrawal(this.rpc, this.userId, amount);
        const sigAction = getWithdrawalSigAction({
            amount: amount,
            nonce: realNonce,
            network_magic: this.l2NetworkMagic,
            user: this.userId,
            l1_address: l1Address,
            withdrawal_fee: WITHDRAWAL_FEE_AMOUNT,
        });

        const sig = await this.zkSignSigAction(sigAction);
        return {
            signature_proof: sig,
            user_id: this.userId,
            nonce: cityFelt(realNonce),
            value: cityFelt(amount),
            destination_type: decoded.scriptTypeFlag,
            destination: u8ArrayToHex(decoded.publicKeyHash),
        };
    }
    async prepareClaimDeposit(
        txidOrDepositId: string,
        signature: string,
        prover: ICitySecp256K1SignatureProver
    ): Promise<ICityClaimDepositRPCRequest> {
        const { hash, deposit } = await this.getClaimDepositMessageHash(txidOrDepositId);
        const proof = await prover.generateSecp256K1SignatureProof(deposit.public_key, signature, hash);
        await userWalletCache.processClaimDeposit(this.rpc, this.userId, deposit.value);
        return {
            signature_proof: proof,
            user_id: this.userId,
            txid: reverseHexBytes(deposit.txid),
            deposit_id: deposit.deposit_id,
            value: deposit.value,
            public_key: deposit.public_key,
        };
    }
    async claimDeposit(
        txidOrDepositId: string,
        signature: string,
        prover: ICitySecp256K1SignatureProver
    ): Promise<void> {
        const request = await this.prepareClaimDeposit(txidOrDepositId, signature, prover);
        await this.rpc.claimDeposit(request);
        await this.refresh();
        this.unprocessedInflows += BigInt(request.value);
    }
    async transfer(recipient: SCNumberLike, amount: SCNumberLike, nonce?: SCNumberLike | undefined): Promise<void> {
        const request = await this.prepareTransfer(recipient, amount, nonce);
        await this.rpc.tokenTransfer(request);
        await this.refresh();
        this.unprocessedOutflows += BigInt(request.value);
    }
    async withdraw(l1Address: string, amount: SCNumberLike, nonce?: SCNumberLike | undefined): Promise<void> {
        const request = await this.prepareWithdrawal(l1Address, amount, nonce);
        await this.rpc.addWithdrawal(request);
        await this.refresh();
        this.unprocessedOutflows += BigInt(request.value);
    }
}

export { CityUserWallet };
