import { getPsyNetworkMagicForNetworkId } from '../action/constants.mjs';
import '../utils/address.mjs';
import '../utils/felt.mjs';
import '../coord-edge-rpc/types.mjs';
import { MultiCoordinatorRpcProvider } from '../coord-edge-rpc/client.mjs';
import '../realm-edge-rpc/types.mjs';
import { MultiRealmRpcProvider } from '../realm-edge-rpc/client.mjs';
import '../utils/json.mjs';
import { PsyMemoryTransactionSignerProvider } from '../zksigner/memory/provider.mjs';
import { PsyUserWallet } from './userWallet.mjs';
import '../local-web-prover/psy_prover.mjs';
import { PsyWasmWebProverProvider } from '../local-web-prover/provider.mjs';

class PsyUserWalletProvider {
    constructor(networkId, coordinatorEdgeRpcProvider, realmEdgeRpcProvider, signerProvider, prover) {
        this.networkId = networkId;
        this.coordinatorEdgeRpcProvider = coordinatorEdgeRpcProvider;
        this.realmEdgeRpcProvider = realmEdgeRpcProvider;
        this.l2NetworkMagic = getPsyNetworkMagicForNetworkId(networkId);
        this.signerProvider = signerProvider;
        this.prover = prover;
    }
    async getUserWallets() {
        const signers = await this.signerProvider.getSigners();
        const publicKeys = await Promise.all(signers.map((signer) => signer.getPublicKeyHex()));
        const userIds = await Promise.all(publicKeys.map(async (publicKey) => {
            try {
                return { userId: await this.coordinatorEdgeRpcProvider.getUserId(publicKey), status: true };
            }
            catch (error) {
                console.warn(`Failed to get user ID for public key ${publicKey}:`, error);
                return { userId: 0, status: false };
            }
        }));
        return userIds.map(({ userId, status }, index) => new PsyUserWallet(this.networkId, signers[index], this.coordinatorEdgeRpcProvider, this.realmEdgeRpcProvider.getRpcProviderByUserId(userId), userId, publicKeys[index], status));
    }
    async getContractState(checkpointId, contractId, userId, slots) {
        const userStateTreeHeight = (await this.coordinatorEdgeRpcProvider.getContractLeafData(contractId)).state_tree_height;
        if (!Array.isArray(slots)) {
            throw new Error(`slots must be an array, got ${typeof slots}`);
        }
        const slotValues = await this.realmEdgeRpcProvider.getRpcProviderByUserId(userId).getSlotValues(checkpointId, userId, contractId, Number(userStateTreeHeight), slots);
        return slotValues;
    }
    async sendTransaction(contractId, functionName, args, publicKey) {
        const signer = await this.signerProvider.getSignerByPublicKeyHex(publicKey);
        const contractCallData = {
            contract_calls: [
                {
                    contract_id: BigInt(contractId),
                    method_name: functionName,
                    inputs: args.map((arg) => BigInt(arg))
                }
            ],
            software_defined_call: {
                "inputs": []
            }
        };
        return signer.signAndSubmit(publicKey, contractCallData);
    }
    async getLatestCheckpointId() {
        const latestState = await this.coordinatorEdgeRpcProvider.getLatestBlockState();
        return latestState.checkpoint_id;
    }
}
async function createMemoryWalletProvider(config) {
    const networkId = "regtest";
    const coordinator_rpc = new MultiCoordinatorRpcProvider(config.coordinator_configs);
    const realm_rpc = new MultiRealmRpcProvider(config.realm_configs, config.users_per_realm);
    const userProver = new PsyWasmWebProverProvider(config);
    console.log("User Prover:", userProver);
    const transactionSignerProvider = new PsyMemoryTransactionSignerProvider(userProver, networkId);
    return new PsyUserWalletProvider(networkId, coordinator_rpc, realm_rpc, transactionSignerProvider, userProver);
}

export { PsyUserWalletProvider, createMemoryWalletProvider };
