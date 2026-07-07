import { getPsyNetworkMagicForNetworkId, NetworkId } from "../../action";
import { ClaimBatchItem, ContractCallArgs, ContractCallData, DPNFunctionCircuitDefinition, GeneratedTxTraceJson, IPsyUserProverProvider, ProveTxTraceResumableJson, SignType, TraceProofConcurrentResult, TraceResumeBlobJson, TxMetadata } from "../../local-prover-rpc";
import { IPsyTransactionSigner, TPsyTransactionSignerAbility } from "../types";
import { PsyJSON } from "../../utils/json";

function isStaleTraceAnchorError(message: string): boolean {
    const normalized = message.toLowerCase();
    return (
        normalized.includes("stale trace anchor") ||
        normalized.includes("start_user_leaf_hash changed while proving")
    );
}

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
        resumeBlob?: TraceResumeBlobJson,
    ): Promise<ProveTxTraceResumableJson> {
        const envelopeObj =
            typeof envelope === "string"
                ? (PsyJSON.parse(envelope) as GeneratedTxTraceJson)
                : (PsyJSON.parse(PsyJSON.stringify(envelope)) as GeneratedTxTraceJson);
        const envelopeJson = PsyJSON.stringify(envelopeObj);
        const toU8 = (a: number[] | Uint8Array): Uint8Array =>
            a instanceof Uint8Array ? a : new Uint8Array(a);
        let latestResumeBlob: Uint8Array | null = resumeBlob ? toU8(resumeBlob) : null;
        let restoreBlob: Uint8Array | undefined = latestResumeBlob ?? undefined;

        try {
            for (;;) {
                const stepResult = await this.prover.proveTraceStep(
                    pkHash,
                    envelopeJson,
                    restoreBlob,
                );
                restoreBlob = undefined;

                if (stepResult.status === "progress") {
                    latestResumeBlob = toU8(stepResult.resume_blob);
                    continue;
                }

                if (stepResult.status === "submitted") {
                    return {
                        generated: envelopeObj,
                        proved: {
                            sig_hash: envelopeObj.sig_hash,
                            tx_hash: stepResult.tx_hash,
                            checkpoint_id: null,
                            status: "submitted",
                        },
                        error: null,
                        status: "submitted",
                        resume_blob: null,
                    };
                }

                latestResumeBlob = stepResult.resume_blob
                    ? toU8(stepResult.resume_blob)
                    : null;
                return {
                    generated: envelopeObj,
                    proved: null,
                    error: PsyJSON.stringify(stepResult.error),
                    status: "failed",
                    resume_blob: stepResult.needs_regenerate
                        ? null
                        : latestResumeBlob,
                };
            }
        } catch (e: any) {
            const message = e?.message ?? String(e);
            return {
                generated: envelopeObj,
                proved: null,
                error: PsyJSON.stringify(message),
                status: "failed",
                resume_blob: isStaleTraceAnchorError(message)
                    ? null
                    : latestResumeBlob,
            };
        }
    }

    async proveTxTraceConcurrent(pkHash: string, envelope: string | GeneratedTxTraceJson): Promise<TraceProofConcurrentResult> {
        return this.prover.proveTraceJobsConcurrent(pkHash, envelope);
    }
}

export { PsyMemoryTransactionSigner };
