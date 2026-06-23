'use strict';

var constants = require('../../action/constants.cjs');
require('../../utils/address.cjs');
require('../../utils/felt.cjs');

class PsyMemoryTransactionSigner {
    constructor(proverProvider, networkId, publicKeyHex, privateKeyHex, signType, fingerprint) {
        this.networkId = networkId;
        this.networkMagic = constants.getPsyNetworkMagicForNetworkId(networkId);
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.prover = proverProvider;
        this.signType = signType;
        this.fingerprint = fingerprint;
    }
    static async create(proverProvider, networkId, privateKeyHex, signType, fingerprint) {
        const publicKeyHex = await proverProvider.addUser(privateKeyHex, signType, fingerprint);
        return new PsyMemoryTransactionSigner(proverProvider, networkId, publicKeyHex, privateKeyHex, signType.toString(), fingerprint);
    }
    getPrivateKeyHex() {
        return Promise.resolve(this.privateKeyHex);
    }
    getSignType() {
        return Promise.resolve(this.signType);
    }
    getFingerprint() {
        return Promise.resolve(this.fingerprint);
    }
    async signAndSubmit(pk_hash, callData) {
        return this.prover.execContractCall(pk_hash, callData);
    }
    async execContractCallWithTrace(pk_hash, callData) {
        return this.prover.execContractCallWithTrace(pk_hash, callData);
    }
    async generateTxTrace(pk_hash, callData) {
        return this.prover.generateTxTrace(pk_hash, callData);
    }
    async proveTxTrace(pk_hash, envelope) {
        return this.prover.proveTxTrace(pk_hash, envelope);
    }
    async proveTxTraceResumable(pk_hash, envelope) {
        return this.prover.proveTxTraceResumable(pk_hash, envelope);
    }
    async deployContract(pk_hash, circuitDefs) {
        return this.prover.deployContract(pk_hash, circuitDefs);
    }
    getAbilities() {
        return ["sign-hash", "export-private-key-hex"];
    }
    async getPublicKeyHex() {
        return this.publicKeyHex;
    }
    async registerUser(privateKeyHex, signType, fingerprint) {
        return this.prover.registerUser(privateKeyHex, signType, fingerprint);
    }
    async addUser(privateKeyHex, signType, fingerprint) {
        return this.prover.addUser(privateKeyHex, signType, fingerprint);
    }
    async getClaimRewardsCallArgs(jobInfos) {
        return this.prover.getClaimRewardsCallArgs(jobInfos);
    }
    async claimRewards(pk_hash, jobInfos) {
        return this.prover.claimRewards(pk_hash, jobInfos);
    }
}

exports.PsyMemoryTransactionSigner = PsyMemoryTransactionSigner;
