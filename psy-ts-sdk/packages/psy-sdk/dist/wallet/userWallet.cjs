'use strict';

var cache = require('./cache.cjs');
var felt = require('../utils/felt.cjs');
require('../utils/json.cjs');
require('../utils/random.cjs');
var constants = require('../action/constants.cjs');
require('../utils/address.cjs');

class PsyUserWallet {
    constructor(networkId, signer, coordinator, realm, userId, publicKeyHex, status) {
        this.networkId = networkId;
        this.networkMagic = constants.getPsyNetworkMagicForNetworkId(this.networkId);
        this.signer = signer;
        this.coordinator = coordinator;
        this.realm = realm;
        this.userId = userId;
        this.publicKeyHex = publicKeyHex;
        this.status = status;
    }
    async refresh() {
        const publicKeyHex = await this.signer.getPublicKeyHex();
        try {
            const userId = await this.coordinator.getUserId(publicKeyHex);
            const { user, cache: cache$1 } = await cache.userWalletCache.refreshUserFull(this.realm, userId);
            user.balance = cache$1.localBalance;
            user.nonce = cache$1.localNonce;
            this.status = true;
            return user;
        }
        catch (e) {
            console.warn("Error refreshing user wallet:", e);
            this.status = false;
            return {
                public_key: this.publicKeyHex,
                user_state_tree_root: this.publicKeyHex,
                balance: BigInt(0),
                nonce: BigInt(0),
                last_checkpoint_id: BigInt(0),
                event_index: BigInt(0),
                user_id: BigInt(0),
            };
        }
    }
    async getUserInfo() {
        const user = await this.refresh();
        const publicKeyHex = await this.signer.getPublicKeyHex();
        return Promise.resolve({
            networkId: this.networkId,
            l2NetworkMagic: this.networkMagic,
            nonce: user.nonce.toString(10),
            balance: user.balance,
            userId: user.user_id,
            publicKeyHex: publicKeyHex,
        });
    }
    async getBalance() {
        const b = await this.refresh();
        return felt.psyFelt(b.balance);
    }
    async getBalanceString() {
        const balance = await this.getBalance();
        return balance.toString();
    }
    // async registerUser(privateKey: PrivateKey): Promise<PublicKey> {
    //     const pk = await this.prover.registerUser(privateKey);
    //     this.accounts.set(pk, privateKey)
    //     return pk
    // }
    // async addUser(privateKey: PrivateKey): Promise<PublicKey> {
    //     const pk = await this.prover.addUser(privateKey);
    //     this.accounts.set(pk, privateKey)
    //     return pk
    // }
    // async switchUser(pkHash: PublicKey): Promise<void> {
    //     await this.prover.switchUser(pkHash);
    //     const sk = this.accounts.get(pkHash);
    //     if (!sk) {
    //         throw new Error("private key not found");
    //     }
    //     this.singer = new PsyMemoryTransactionSigner(this.prover, pkHash, sk);
    //     const userId = await this.coordinator.getUserId(pkHash);
    //     this.realm.setUserId(userId)
    // }
    // async getZKPublicKey(): Promise<PublicKey> {
    //     return this.signer.getPublicKeyHex();
    // }
    // async importPrivateKey(privateKey: PrivateKey): Promise<PublicKey> {
    //     return this.prover.addUser(privateKey);
    // }
    // async getRandomKeypair(): Promise<WalletKeyPair> {
    //     return this.prover.getRandomKeypair();
    // }
    async deployContract(pk_hash, circuitDefs) {
        return this.signer.deployContract(pk_hash, circuitDefs);
    }
    // async getDeployContract(circuitDefs: DPNFunctionCircuitDefinition[]): Promise<QBCDeployContract> {
    //     // await this.prover.switchUser(await this.getZKPublicKey());
    //     // await this.prover.startSession();
    //     return this.prover.getDeployContractCmd(circuitDefs);
    // }
    async execContractCall(pk_hash, contractCallArgs) {
        const callData = {
            contract_calls: Array.isArray(contractCallArgs) ? contractCallArgs : [contractCallArgs],
            software_defined_call: { "inputs": [] }
        };
        return this.signer.signAndSubmit(pk_hash, callData);
    }
    async execContractCallWithTrace(pk_hash, contractCallArgs) {
        const callData = {
            contract_calls: Array.isArray(contractCallArgs) ? contractCallArgs : [contractCallArgs],
            software_defined_call: { "inputs": [] }
        };
        return this.signer.execContractCallWithTrace(pk_hash, callData);
    }
    // Produce a savable trace envelope without proving/submitting. The wallet persists the
    // returned envelope (keyed by `sig_hash`) and later proves/tracks it via the step API.
    async generateTxTrace(pk_hash, contractCallArgs) {
        const callData = {
            contract_calls: Array.isArray(contractCallArgs) ? contractCallArgs : [contractCallArgs],
            software_defined_call: { "inputs": [] }
        };
        return this.signer.generateTxTrace(pk_hash, callData);
    }
    // Batch-claim variant of generateTxTrace: returns the same savable envelope, proven/tracked
    // via the shared step-proving path below.
    async generateBatchClaimTxTrace(pk_hash, claims) {
        return this.signer.generateBatchClaimTxTrace(pk_hash, claims);
    }
    async proveTxTraceStep(pk_hash, envelope, resumeFrom) {
        return this.signer.proveTxTraceStep(pk_hash, envelope, resumeFrom);
    }
}

exports.PsyUserWallet = PsyUserWallet;
