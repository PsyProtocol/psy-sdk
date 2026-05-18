import {
    DPNFunctionCircuitDefinition,
    IPsyUserProverProvider,
    QBCDeployContract,
    uploadPendingContractAbi,
} from "@psy/psy-sdk";

export interface DeployContractWithAbiOptions {
    prover: IPsyUserProverProvider;
    deployer: string;
    circuitDefinitions: DPNFunctionCircuitDefinition[];
    servicesUrl: string;
    abi: unknown;
    metadata?: Record<string, unknown>;
}

export interface DeployContractWithAbiResult {
    deployHash: string;
    deployContract: QBCDeployContract;
}

export async function deployContractWithAbi({
    prover,
    deployer,
    circuitDefinitions,
    servicesUrl,
    abi,
    metadata,
}: DeployContractWithAbiOptions): Promise<DeployContractWithAbiResult> {
    const deployContract = await prover.getDeployContractCmd(deployer, circuitDefinitions);
    await uploadPendingContractAbi(servicesUrl, {
        deployContract,
        abi,
        metadata,
        deployer,
    });
    const deployHash = prover.submitDeployContractCmd
        ? await prover.submitDeployContractCmd(deployContract)
        : await prover.deployContract(deployer, circuitDefinitions);
    return {
        deployHash,
        deployContract,
    };
}
