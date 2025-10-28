import {
    DogeLinkElectrsRPC,
    DogeLinkRPC,
    DogeNetworkId,
    IAddressStatsResponse,
    IBasicBlock,
    IBlockStatus,
    IDogeLinkElectrsRPC,
    IDogeLinkRPC,
    IDogeNetwork,
    IGetTXResponse,
    IMempoolRecentTransaction,
    IMempoolStatus,
    IScriptHashStatsResponse,
    ITransactionOutSpend,
    IUTXO,
    Transaction,
} from "doge-sdk";
import { IWalletWidgetRPC } from "./types";
import { Block } from "doge-sdk/dist/types/block";
import { IFeeEstimateMap } from "doge-sdk/dist/types/rpc/types";
class WalletWidgetRPC implements IWalletWidgetRPC {
    rpc?: DogeLinkRPC = undefined;
    electrsRPC: IDogeLinkElectrsRPC;
    networkId: DogeNetworkId;
    explorerURL = "http://localhost:1337/explorer";
    constructor(networkId: DogeNetworkId, electrsURL: string, dogeRPCURL?: string) {
        this.networkId = networkId;
        this.electrsRPC = new DogeLinkElectrsRPC(electrsURL, networkId);
        if (dogeRPCURL) {
            const url = new URL(dogeRPCURL);
            if (dogeRPCURL.indexOf("network=") == -1) {
                url.searchParams.append("network", networkId);
            }
            this.rpc = new DogeLinkRPC(url.toString());
        }
    }
    async getFeeEstimateMap(): Promise<IFeeEstimateMap> {
        const base = await this.electrsRPC.getFeeEstimateMap();
        if (Object.keys(base).length) {
            return base;
        } else if (this.rpc) {
            return this.rpc!.getFeeEstimateMap();
        } else {
            throw new Error("No fee estimate map available");
        }
    }
    estimateSmartFee(target: number): Promise<number> {
        return this.getFastRPC().estimateSmartFee(target);
    }
    getBlockStatus(hash: string): Promise<IBlockStatus> {
        return this.electrsRPC.getBlockStatus(hash);
    }
    getBlockGroup(start?: number | undefined): Promise<IBasicBlock[]> {
        return this.electrsRPC.getBlockGroup(start);
    }
    getBlockBasic(hash: string): Promise<IBasicBlock> {
        return this.electrsRPC.getBlockBasic(hash);
    }
    getBalance(address: string): Promise<number> {
        return this.electrsRPC.getBalance(address);
    }
    getTransactionsFor(
        addressOrScriptHash: string,
        confirmed?: boolean | undefined,
        afterTxid?: string | undefined
    ): Promise<IGetTXResponse[]> {
        return this.electrsRPC.getTransactionsFor(addressOrScriptHash, confirmed, afterTxid);
    }
    getStatsFor(addressOrScriptHash: string): Promise<IAddressStatsResponse | IScriptHashStatsResponse> {
        return this.electrsRPC.getStatsFor(addressOrScriptHash);
    }
    getBlockHeight(): Promise<number> {
        return this.electrsRPC.getBlockHeight();
    }
    getTransactionElectrs(txId: string): Promise<IGetTXResponse>;
    getTransactionElectrs(txId: string, rawHex: true): Promise<string>;
    getTransactionElectrs(txId: string, rawHex: false): Promise<IGetTXResponse>;
    getTransactionElectrs(txId: string, rawHex: undefined): Promise<IGetTXResponse>;
    getTransactionElectrs(txId: string, rawHex?: boolean | undefined): Promise<string> | Promise<IGetTXResponse> {
        return this.electrsRPC.getTransactionElectrs(txId, rawHex as any);
    }
    getUTXOs(addressOrScriptHash: string): Promise<IUTXO[]> {
        return this.electrsRPC.getUTXOs(addressOrScriptHash);
    }
    waitUntilUTXO(
        address: string,
        pollInterval?: number | undefined,
        maxAttempts?: number | undefined
    ): Promise<IUTXO[]> {
        return this.electrsRPC.waitUntilUTXO(address, pollInterval, maxAttempts);
    }
    getMempoolStatus(): Promise<IMempoolStatus> {
        return this.electrsRPC.getMempoolStatus();
    }
    getMempoolRecentTransactions(): Promise<IMempoolRecentTransaction[]> {
        return this.electrsRPC.getMempoolRecentTransactions();
    }
    getTransactionOutSpends(txid: string): Promise<ITransactionOutSpend[]> {
        return this.electrsRPC.getTransactionOutSpends(txid);
    }
    getFastRPC(): IDogeLinkRPC {
        if (this.rpc) {
            return this.rpc;
        } else {
            return this.electrsRPC;
        }
    }
    sendFromWallet(address: string, amount: string | number, walletName?: string | undefined): Promise<string> {
        return this.rpc!.sendFromWallet(address, amount, walletName);
    }
    canSendFromWallet(): boolean {
        return !!this.rpc;
    }
    getNetwork(): IDogeNetwork {
        return this.electrsRPC.getNetwork();
    }
    getBlockCount(): Promise<number> {
        return this.getFastRPC().getBlockCount();
    }
    getRawTransaction(txId: string): Promise<string> {
        return this.getFastRPC().getRawTransaction(txId);
    }
    getTransaction(txId: string): Promise<Transaction> {
        return this.getFastRPC().getTransaction(txId);
    }
    getBlockHash(height: number): Promise<string> {
        return this.getFastRPC().getBlockHash(height);
    }
    mineBlocks(count: number, address?: string | undefined): Promise<string[]> {
        return this.getFastRPC().mineBlocks(count, address);
    }
    isDoge(): boolean {
        return this.electrsRPC.isDoge();
    }
    sendRawTransaction(txHex: string): Promise<string> {
        return this.getFastRPC().sendRawTransaction(txHex);
    }
    getBlock(blockHashOrNumber: string | number): Promise<Block> {
        return this.getFastRPC().getBlock(blockHashOrNumber);
    }
    getBlocks(start: number, count: number): Promise<Block[]> {
        return this.getFastRPC().getBlocks(start, count);
    }
    resolveBlockHash(blockHashOrNumber: string | number): Promise<string> {
        return this.getFastRPC().resolveBlockHash(blockHashOrNumber);
    }
    resolveBlockNumber(blockHashOrNumber: string | number): Promise<number> {
        return this.getFastRPC().resolveBlockNumber(blockHashOrNumber);
    }

    async waitForTx(txid: string, waitDuration: number = 1000, maxAttempts = 9999): Promise<IGetTXResponse> {
        for (let i = 0; i < maxAttempts; i++) {
            try {
                const tx = await this.getTransactionElectrs(txid);
                return tx;
            } catch (e) {}
            await new Promise((resolve) => setTimeout(resolve, waitDuration));
        }
        throw new Error("Transaction not found after " + maxAttempts + " attempts");
    }
    getTXURL(txid: string) {
        return this.explorerURL + "/tx/" + txid;
    }
}

export { WalletWidgetRPC };
