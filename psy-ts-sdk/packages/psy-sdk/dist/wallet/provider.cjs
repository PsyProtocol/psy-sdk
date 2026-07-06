'use strict';

var constants = require('../action/constants.cjs');
require('../utils/address.cjs');
require('../utils/felt.cjs');
require('../coord-edge-rpc/types.cjs');
var client = require('../coord-edge-rpc/client.cjs');
require('../realm-edge-rpc/types.cjs');
var client$1 = require('../realm-edge-rpc/client.cjs');
require('../utils/json.cjs');
var provider$1 = require('../zksigner/memory/provider.cjs');
var userWallet = require('./userWallet.cjs');
require('../local-web-prover/psy_prover.cjs');
var provider = require('../local-web-prover/provider.cjs');

class PsyUserWalletProvider {
    constructor(networkId, coordinatorEdgeRpcProvider, realmEdgeRpcProvider, signerProvider, prover) {
        this.networkId = networkId;
        this.coordinatorEdgeRpcProvider = coordinatorEdgeRpcProvider;
        this.realmEdgeRpcProvider = realmEdgeRpcProvider;
        this.l2NetworkMagic = constants.getPsyNetworkMagicForNetworkId(networkId);
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
        return userIds.map(({ userId, status }, index) => new userWallet.PsyUserWallet(this.networkId, signers[index], this.coordinatorEdgeRpcProvider, this.realmEdgeRpcProvider.getRpcProviderByUserId(userId), userId, publicKeys[index], status));
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
    const coordinator_rpc = new client.MultiCoordinatorRpcProvider(config.coordinator_configs);
    const realm_rpc = new client$1.MultiRealmRpcProvider(config.realm_configs, config.users_per_realm);
    const userProver = new provider.PsyWasmWebProverProvider(config);
    console.log("User Prover:", userProver);
    const transactionSignerProvider = new provider$1.PsyMemoryTransactionSignerProvider(userProver, networkId);
    return new PsyUserWalletProvider(networkId, coordinator_rpc, realm_rpc, transactionSignerProvider, userProver);
}

exports.PsyUserWalletProvider = PsyUserWalletProvider;
exports.createMemoryWalletProvider = createMemoryWalletProvider;
