import { getPsyNetworkMagicForNetworkId, NetworkId } from "../../action";
import { ClaimBatchItem, ContractCallArgs, ContractCallData, DPNFunctionCircuitDefinition, GeneratedTxTraceJson, IPsyUserProverProvider, ProveTxTraceResumableJson, SignType, TraceProofConcurrentResult, TxMetadata } from "../../local-prover-rpc";
import { IPsyTransactionSigner, TPsyTransactionSignerAbility } from "../types";
import { PsyJSON } from "../../utils/json";

class PsyMemoryTransactionSigner implements IPsyTransactionSigner {
    networkId: NetworkId;
    networkMagic: bigint;
    publicKeyHex: string;
    privateKeyHex: string;
    signType: SignType;
    fingerprint: string;
    prover: IPsyUserProverProvider;
    private constructor(
        proverProvider: IPsyUserProverProvider,
        networkId: NetworkId,
        publicKeyHex: string,
        privateKeyHex: string,
        signType: SignType,
        fingerprint: string
    ) {
        this.networkId = networkId;
        this.networkMagic = getPsyNetworkMagicForNetworkId(networkId);
        this.publicKeyHex = publicKeyHex;
        this.privateKeyHex = privateKeyHex;
        this.prover = proverProvider;
        this.signType = signType;
        this.fingerprint = fingerprint;
    }
    static async create(proverProvider: IPsyUserProverProvider, networkId: NetworkId, privateKeyHex: string, signType: SignType, fingerprint: string) {
        const publicKeyHex = await proverProvider.addUser(privateKeyHex, signType, fingerprint);
        return new PsyMemoryTransactionSigner(proverProvider, networkId, publicKeyHex, privateKeyHex, signType, fingerprint);
    }
    getPrivateKeyHex(): Promise<string> {
        return Promise.resolve(this.privateKeyHex);
    }

    getSignType(): Promise<string> {
        return Promise.resolve(this.signType);
    }

    getFingerprint(): Promise<string> {
        return Promise.resolve(this.fingerprint);
    }

    async signAndSubmit(pk_hash: string, callData: ContractCallData): Promise<string> {
        return this.prover.execContractCall(pk_hash, callData);
    }

    async execContractCallWithTrace(pk_hash: string, callData: ContractCallData): Promise<TxMetadata> {
        return this.prover.execContractCallWithTrace(pk_hash, callData);
    }

    async generateTxTrace(pk_hash: string, callData: ContractCallData): Promise<GeneratedTxTraceJson> {
        return this.prover.generateTxTrace(pk_hash, callData);
    }

    async generateBatchClaimTxTrace(pk_hash: string, claims: ClaimBatchItem[]): Promise<GeneratedTxTraceJson> {
        return this.prover.generateBatchClaimTxTrace(pk_hash, claims);
    }



    async deployContract(pk_hash: string, circuitDefs: DPNFunctionCircuitDefinition[]): Promise<string> {
        return this.prover.deployContract(pk_hash, circuitDefs);
    }

    getAbilities(): TPsyTransactionSignerAbility[] {
        return ["sign-hash", "export-private-key-hex"];
    }

    async getPublicKeyHex(): Promise<string> {
        return this.publicKeyHex;
    }

    async registerUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string> {
        return this.prover.registerUser(privateKeyHex, signType, fingerprint);
    }

    async addUser(privateKeyHex: string, signType: SignType, fingerprint?: string): Promise<string> {
        return this.prover.addUser(privateKeyHex, signType, fingerprint);
    }

    async getClaimRewardsCallArgs(jobInfos: string): Promise<ContractCallArgs[]> {
        return this.prover.getClaimRewardsCallArgs(jobInfos);
    }

    async claimRewards(pk_hash: string, jobInfos: string): Promise<string> {
        return this.prover.claimRewards(pk_hash, jobInfos);
    }

    async proveTxTraceStep(
        pkHash: string,
        envelope: string | GeneratedTxTraceJson,
        resumeFrom?: {
            proof_tree_meta: unknown;
            last_step_info: unknown;
            current_header: unknown;
            previous_header: unknown;
            proof_blobs: Uint8Array[];
            next_step_index: number;
        },
    ): Promise<ProveTxTraceResumableJson> {
        let envelopeObj =
            typeof envelope === "string"
                ? (PsyJSON.parse(envelope) as GeneratedTxTraceJson)
                : (PsyJSON.parse(PsyJSON.stringify(envelope)) as GeneratedTxTraceJson);
        let envelopeJson = PsyJSON.stringify(envelopeObj);

        const toU8 = (a: number[] | Uint8Array): Uint8Array => a instanceof Uint8Array ? a : new Uint8Array(a);
        const toArray = (a: Uint8Array): number[] => Array.from(a);
        const tracePayload = PsyJSON.parse(envelopeObj.trace.payload) as {
            ups_start_witness: Record<string, unknown>;
            steps: Array<Record<string, unknown>>;
        };
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
            let meta: unknown;
            let baton: unknown;
            let currHeader: unknown;
            let prevHeader: unknown;
            let allProofBlobs: Uint8Array[];
            let startStep: number;

            if (resumeFrom) {
                meta = resumeFrom.proof_tree_meta;
                baton = resumeFrom.last_step_info;
                currHeader = resumeFrom.current_header;
                prevHeader = resumeFrom.previous_header;
                allProofBlobs = resumeFrom.proof_blobs.map(toU8);
                startStep = resumeFrom.next_step_index;
            } else {
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
                    const externalProof = toU8((step?.proof as number[] | Uint8Array | undefined) ?? []);
                    if (!externalProof.length) {
                        throw new Error(`trace external proof ${stepIndex} missing proof bytes`);
                    }
                    meta = await this.prover.insertExternalProof(
                        pkHash,
                        envelopeJson,
                        meta,
                        baton,
                        currHeader,
                        prevHeader,
                        String(step?.fingerprint ?? ""),
                        externalProof,
                    );
                    allProofBlobs.push(externalProof);
                    syncEnvelopePayload();
                    continue;
                }

                const stepResult = await this.prover.proveTraceStep(
                    pkHash,
                    envelopeJson,
                    stepIndex,
                    meta,
                    baton,
                    currHeader,
                    prevHeader,
                );
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
            const signatureProof = await this.prover.signSighash(
                this.publicKeyHex,
                sighashJson,
                envelopeJson,
                currHeader,
            );
            const endCapResult = await this.prover.proveEndCapProof(
                pkHash,
                envelopeJson,
                meta,
                baton,
                allProofBlobs,
                signatureProof,
            );
            const txHash = await this.prover.submitEndCap(
                envelopeJson,
                toU8(endCapResult.end_cap_proof),
            );

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
        } catch (e: any) {
            return {
                generated: envelopeObj,
                proved: null,
                error: PsyJSON.stringify(e?.message ?? String(e)),
                status: "failed",
            };
        }
    }

    async proveTxTraceConcurrent(pkHash: string, envelope: string | GeneratedTxTraceJson): Promise<TraceProofConcurrentResult> {
        return this.prover.proveTraceJobsConcurrent(pkHash, envelope);
    }
}

export { PsyMemoryTransactionSigner };
