import { FetchHTTPClient, IHTTPClient } from "../http";
import type { QBCDeployContract } from "../types";
import { PsyJSON } from "../utils";

export interface PendingContractAbiUploadInput {
    contentHash?: string;
    content_hash?: string;
    deployContract?: QBCDeployContract;
    deploy_contract?: QBCDeployContract;
    abi: unknown;
    metadata?: Record<string, unknown>;
    deployer?: string;
}

export interface PendingContractAbiUploadResult {
    content_hash: string;
    status: string;
}

interface ApiEnvelope<T> {
    success?: boolean;
    data?: T;
    error?: string;
}

export async function uploadPendingContractAbi(
    servicesUrl: string,
    input: PendingContractAbiUploadInput,
    httpClient: IHTTPClient = new FetchHTTPClient()
): Promise<PendingContractAbiUploadResult> {
    const contentHash = input.contentHash ?? input.content_hash;
    const deployContract = input.deployContract ?? input.deploy_contract;
    if (!contentHash && !deployContract) {
        throw new Error("uploadPendingContractAbi requires contentHash or deployContract");
    }

    const response = await httpClient.sendRequest({
        url: `${servicesUrl.replace(/\/$/, "")}/api/v1/contract/abi/pending`,
        method: "POST",
        headers: { "content-type": "application/json" },
        responseType: "json",
        body: PsyJSON.stringify({
            content_hash: contentHash,
            deploy_contract: deployContract,
            abi: input.abi,
            metadata: input.metadata,
            deployer: input.deployer,
        }),
    });

    const envelope = response.body as ApiEnvelope<PendingContractAbiUploadResult>;
    if (response.statusCode < 200 || response.statusCode >= 300 || envelope?.success === false) {
        throw new Error(
            `failed to upload pending contract ABI: HTTP ${response.statusCode} ${envelope?.error ?? PsyJSON.stringify(response.body)}`
        );
    }
    if (!envelope?.data) {
        throw new Error("pending contract ABI upload response missing data");
    }
    return envelope.data;
}
