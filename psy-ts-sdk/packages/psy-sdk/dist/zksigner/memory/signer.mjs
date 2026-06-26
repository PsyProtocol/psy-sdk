import { getPsyNetworkMagicForNetworkId } from '../../action/constants.mjs';
import '../../utils/address.mjs';
import '../../utils/felt.mjs';
import { PsyJSON } from '../../utils/json.mjs';

class PsyMemoryTransactionSigner {
    constructor(proverProvider, networkId, publicKeyHex, privateKeyHex, signType, fingerprint) {
        this.networkId = networkId;
        this.networkMagic = getPsyNetworkMagicForNetworkId(networkId);
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.prover = proverProvider;
        this.signType = signType;
        this.fingerprint = fingerprint;
    }
    static async create(proverProvider, networkId, privateKeyHex, signType, fingerprint) {
        const publicKeyHex = await proverProvider.addUser(privateKeyHex, signType, fingerprint);
        return new PsyMemoryTransactionSigner(proverProvider, networkId, publicKeyHex, privateKeyHex, signType, fingerprint);
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
    async generateBatchClaimTxTrace(pk_hash, claims) {
        return this.prover.generateBatchClaimTxTrace(pk_hash, claims);
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
    async proveTxTraceStep(pkHash, envelope, resumeFrom) {
        let envelopeObj = typeof envelope === "string"
            ? PsyJSON.parse(envelope)
            : PsyJSON.parse(PsyJSON.stringify(envelope));
        let envelopeJson = PsyJSON.stringify(envelopeObj);
        const toU8 = (a) => a instanceof Uint8Array ? a : new Uint8Array(a);
        const toArray = (a) => Array.from(a);
        const tracePayload = PsyJSON.parse(envelopeObj.trace.payload);
        const syncEnvelopePayload = () => {
            envelopeObj = {
                ...envelopeObj,
                trace: {
                    ...envelopeObj.trace,
                    payload: PsyJSON.stringify(tracePayload),
                },
            };
            envelopeJson = PsyJSON.stringify(envelopeObj);
        };
        try {
            let meta;
            let baton;
            let currHeader;
            let prevHeader;
            let allProofBlobs;
            let startStep;
            if (resumeFrom) {
                meta = resumeFrom.proof_tree_meta;
                baton = resumeFrom.last_step_info;
                currHeader = resumeFrom.current_header;
                prevHeader = resumeFrom.previous_header;
                allProofBlobs = resumeFrom.proof_blobs.map(toU8);
                startStep = resumeFrom.next_step_index;
            }
            else {
                const startResult = await this.prover.proveUpsStart(pkHash, envelopeJson);
                meta = startResult.proof_tree_meta;
                baton = startResult.last_step_info;
                currHeader = startResult.current_header;
                prevHeader = startResult.previous_header;
                const upsProof = toU8(startResult.ups_proof);
                allProofBlobs = [upsProof];
                startStep = 0;
                tracePayload.ups_start_witness = {
                    ...tracePayload.ups_start_witness,
                    proof: { proof: toArray(upsProof) },
                };
                syncEnvelopePayload();
            }
            for (let stepIndex = startStep; stepIndex < tracePayload.steps.length; stepIndex++) {
                const step = tracePayload.steps[stepIndex];
                const kind = String(step?.kind ?? "");
                if (kind === "zk_sign") {
                    break;
                }
                if (kind === "external_proof") {
                    const externalProof = toU8(step?.proof ?? []);
                    if (!externalProof.length) {
                        throw new Error(`trace external proof ${stepIndex} missing proof bytes`);
                    }
                    meta = await this.prover.insertExternalProof(pkHash, envelopeJson, meta, baton, currHeader, prevHeader, String(step?.fingerprint ?? ""), externalProof);
                    allProofBlobs.push(externalProof);
                    syncEnvelopePayload();
                    continue;
                }
                const stepResult = await this.prover.proveTraceStep(pkHash, envelopeJson, stepIndex, meta, baton, currHeader, prevHeader);
                meta = stepResult.proof_tree_meta;
                baton = stepResult.last_step_info;
                currHeader = stepResult.current_header;
                prevHeader = stepResult.previous_header;
                const cfcProof = toU8(stepResult.cfc_proof);
                const upsProof = toU8(stepResult.ups_proof);
                allProofBlobs.push(cfcProof);
                allProofBlobs.push(upsProof);
                tracePayload.steps[stepIndex] = {
                    ...step,
                    proof: {
                        cfc_proof: toArray(cfcProof),
                        ups_proof: toArray(upsProof),
                    },
                };
                syncEnvelopePayload();
            }
            const sighashJson = await this.prover.computeSighashFromEnvelope(envelopeJson, currHeader);
            const signatureProof = await this.prover.signSighash(this.publicKeyHex, sighashJson, envelopeJson, currHeader);
            const endCapResult = await this.prover.proveEndCapProof(pkHash, envelopeJson, meta, baton, allProofBlobs, signatureProof);
            const txHash = await this.prover.submitEndCap(envelopeJson, toU8(endCapResult.end_cap_proof));
            return {
                generated: envelopeObj,
                proved: {
                    sig_hash: envelopeObj.sig_hash,
                    tx_hash: txHash,
                    checkpoint_id: null,
                    status: "submitted",
                },
                error: null,
                status: "submitted",
            };
        }
        catch (e) {
            return {
                generated: envelopeObj,
                proved: null,
                error: PsyJSON.stringify(e?.message ?? String(e)),
                status: "failed",
            };
        }
    }
}

export { PsyMemoryTransactionSigner };
