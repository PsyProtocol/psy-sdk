// src/config/index.ts
import dotenv from 'dotenv';
import { psyConfig } from '../../../../config';

// Load environment variables
dotenv.config();

// Helper function to derive public key from private key
// In production, use proper cryptographic libraries
function derivePublicKey(privateKey: string): string {
    // This is a mock implementation - replace with actual key derivation
    return '0x' + privateKey.substring(0, 40);
}

// Helper to create key pair info
interface KeyPairInfo {
    privateKey: string;
    publicKey: string;
    userId: bigint;
    realmId: number;
}

function createKeyPairInfo(privateKey: string, userId: bigint, realmId: number = 0): KeyPairInfo {
    return {
        privateKey,
        publicKey: derivePublicKey(privateKey),
        userId,
        realmId
    };
}

export const config = {
    rpc: {
        // Default Realm RPC URL (for state queries)
        url: process.env.REALM_RPC_URL || 'http://127.0.0.1:8546',

        // Coordinator RPC URL (for global operations)
        coordinatorUrl: process.env.COORDINATOR_RPC_URL || 'http://127.0.0.1:8545',

        // Chain and timeout settings
        chainId: parseInt(process.env.RPC_CHAIN_ID || '1'),
        timeout: parseInt(process.env.RPC_TIMEOUT || '30000'),

        // Multiple endpoints for different environments
        endpoints: {
            // Local environment endpoints
            local: {
                realm: process.env.REALM_RPC_URL || 'http://127.0.0.1:8546',
                coordinator: process.env.COORDINATOR_RPC_URL || 'http://127.0.0.1:8545',
                // Additional realm endpoints for different realm IDs
                realms: {
                    0: 'http://127.0.0.1:8546',        // Default realm (0)
                    32: 'http://127.0.0.1:8547',    // Realm 1
                    // 8192: 'http://127.0.0.1:8548',  // Realm 8192 (commented in Makefile)
                }
            },
            // Testnet environment endpoints
            testnet: {
                realm: process.env.TESTNET_REALM_RPC_URL || 'https://testnet-realm.example.com',
                coordinator: process.env.TESTNET_COORDINATOR_RPC_URL || 'https://testnet-coordinator.example.com',
                realms: {}
            },
            // Mainnet environment endpoints
            mainnet: {
                realm: process.env.MAINNET_REALM_RPC_URL || 'https://mainnet-realm.example.com',
                coordinator: process.env.MAINNET_COORDINATOR_RPC_URL || 'https://mainnet-coordinator.example.com',
                realms: {}
            }
        }
    },

    contract: {
        id: BigInt(process.env.CONTRACT_ID || '0'),
        userId: BigInt(process.env.USER_ID || '0'),
        // Contract state height from Makefile
        stateHeight: parseInt(process.env.CONTRACT_STATE_HEIGHT || '32'),
    },

    checkpoint: {
        // Default checkpoint ID from Makefile
        defaultId: BigInt(process.env.CHECKPOINT_ID || '100'),
    },

    // User configuration with signer support
    user: {
        // Raw private keys (for backward compatibility)
        privateKeys: {
            user0: process.env.USER0_PRIVATE_KEY || '17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a',
            user1: process.env.USER1_PRIVATE_KEY || '73ae514d6f69510ad778a05128d980951d9d8c097beb022471b2f50f19c41268',
            user32_0: process.env.USER32_0_PRIVATE_KEY || 'f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d',
            user32_1: process.env.USER32_1_PRIVATE_KEY || '88ebebcea0bdfbe88ff0ed470d44242c149343a9ec79244ff829042a62e8ad2d',
        },

        // Key pairs with derived public keys for signer pattern
        keyPairs: {
            user0: createKeyPairInfo(
                process.env.USER0_PRIVATE_KEY || '17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a',
                0n,
                0
            ),
            user1: createKeyPairInfo(
                process.env.USER1_PRIVATE_KEY || '73ae514d6f69510ad778a05128d980951d9d8c097beb022471b2f50f19c41268',
                1n,
                0
            ),
            user32_0: createKeyPairInfo(
                process.env.USER32_0_PRIVATE_KEY || 'f07f91a0bdc0df4ec763285ba0eb578cb6e7a0811c3150494ab54e56f761fc1d',
                BigInt(psyConfig.network.users_per_realm), // User ID in realm 1 (from root config)
                32
            ),
            user32_1: createKeyPairInfo(
                process.env.USER32_1_PRIVATE_KEY || '88ebebcea0bdfbe88ff0ed470d44242c149343a9ec79244ff829042a62e8ad2d',
                536870913n, // User ID in realm 1
                32
            ),
        },

        // Current active user
        currentPrivateKey: process.env.CURRENT_USER_PRIVATE_KEY || process.env.USER0_PRIVATE_KEY,

        // Current active key pair
        get currentKeyPair(): KeyPairInfo {
            const currentPrivateKey = process.env.CURRENT_USER_PRIVATE_KEY || config.user.privateKeys.user0;
            // Find matching key pair or create new one
            const matchingPair = Object.values(config.user.keyPairs).find(
                kp => kp.privateKey === currentPrivateKey
            );
            return matchingPair || createKeyPairInfo(currentPrivateKey, config.contract.userId, config.realm.defaultId);
        }
    },

    // Signer configuration
    signer: {
        // Key management mode
        keyManagementMode: process.env.KEY_MANAGEMENT_MODE || 'local', // 'local' | 'kms' | 'hardware'

        // Signing method
        signingMethod: process.env.SIGNING_METHOD || 'mock', // 'mock' | 'ecdsa' | 'eddsa' | 'custom'

        // Auto-attach signer to contracts
        autoAttachSigner: process.env.AUTO_ATTACH_SIGNER !== 'false',

        // Default signer selection strategy
        defaultSignerStrategy: process.env.DEFAULT_SIGNER_STRATEGY || 'current', // 'current' | 'userId' | 'manual'

        // Transaction signing options
        transactionOptions: {
            // Auto-confirm transactions
            autoConfirm: process.env.TX_AUTO_CONFIRM === 'true',
            // Max retries for failed transactions
            maxRetries: parseInt(process.env.TX_MAX_RETRIES || '3'),
            // Retry delay in ms
            retryDelay: parseInt(process.env.TX_RETRY_DELAY || '1000'),
        }
    },

    // Realm configuration
    realm: {
        defaultId: parseInt(process.env.REALM_ID || '0'),
        // Realm-specific configurations from Makefile
        configs: {
            32: {
                nodeId: 2,
                workerQueueSuffix: 'rwq1',
                notificationsQueueSuffix: 'rnq1',
                proofStoreKeySuffix: 'RP1',
                dbPath: './db/realm1',
                redisUri: 'redis://127.0.0.1:6381',
            },
            // 8192: {
            //     nodeId: 2,
            //     workerQueueSuffix: 'rwq8192',
            //     notificationsQueueSuffix: 'rnq8192',
            //     proofStoreKeySuffix: 'RP8192',
            //     dbPath: './db/realm8192',
            //     redisUri: 'redis://127.0.0.1:6382',
            // }
        }
    },

    // Logging configuration
    logging: {
        enabled: process.env.LOG_ENABLED !== 'false',
        level: process.env.LOG_LEVEL || 'info',
        // Log signer operations
        logSignerOps: process.env.LOG_SIGNER_OPS !== 'false',
        // Detailed log level from Makefile
        rustLogLevel: process.env.RUST_LOG || 'psy_user_cli=debug,psy_dev_cli=debug,psy_node_cli=debug,psy_node=debug,psy_common_circuit=debug,psy_network_circuit=debug,psy_prover=debug,psy_data=debug,plonky2=error',
    }
};

// Helper to get Realm RPC URL based on environment and realm ID
export function getRealmRpcUrl(
    environment: 'local' | 'testnet' | 'mainnet' = 'local',
    realmId?: number
): string {
    const envConfig = config.rpc.endpoints[environment];

    if (realmId !== undefined && envConfig.realms && envConfig.realms[realmId]) {
        return envConfig.realms[realmId];
    }

    return envConfig.realm;
}

// Helper to get Coordinator RPC URL based on environment
export function getCoordinatorRpcUrl(
    environment: 'local' | 'testnet' | 'mainnet' = 'local'
): string {
    return config.rpc.endpoints[environment].coordinator;
}

// Helper to get both RPC URLs for an environment
export function getRpcUrls(
    environment: 'local' | 'testnet' | 'mainnet' = 'local',
    realmId?: number
): { realm: string; coordinator: string } {
    return {
        realm: getRealmRpcUrl(environment, realmId),
        coordinator: getCoordinatorRpcUrl(environment),
    };
}

// Helper to get realm configuration
export function getRealmConfig(realmId: number) {
    return config.realm.configs[realmId] || null;
}

// Helper to get key pair for a specific user
export function getKeyPairForUser(userId: bigint): KeyPairInfo | undefined {
    return Object.values(config.user.keyPairs).find(kp => kp.userId === userId);
}

// Helper to get key pair by public key
export function getKeyPairByPublicKey(publicKey: string): KeyPairInfo | undefined {
    return Object.values(config.user.keyPairs).find(kp => kp.publicKey === publicKey);
}

// Helper to get all key pairs for a realm
export function getKeyPairsForRealm(realmId: number): KeyPairInfo[] {
    return Object.values(config.user.keyPairs).filter(kp => kp.realmId === realmId);
}

// Export types for better TypeScript support
export type Environment = 'local' | 'testnet' | 'mainnet';
export type RealmId = 0 | 32 | 8192;
export type KeyManagementMode = 'local' | 'kms' | 'hardware';
export type SigningMethod = 'mock' | 'ecdsa' | 'eddsa' | 'custom';
export type SignerStrategy = 'current' | 'userId' | 'manual';

export { KeyPairInfo };

// Validation helper
export function validateConfig() {
    const errors: string[] = [];
    const warnings: string[] = [];

    if (!config.rpc.url) {
        errors.push('Missing REALM_RPC_URL');
    }

    if (!config.rpc.coordinatorUrl) {
        errors.push('Missing COORDINATOR_RPC_URL');
    }

    if (config.contract.id === 0n && process.env.NODE_ENV === 'production') {
        warnings.push('CONTRACT_ID is set to 0 in production');
    }

    // Validate signer configuration
    if (config.signer.keyManagementMode === 'local' && process.env.NODE_ENV === 'production') {
        warnings.push('Using local key management in production is not recommended');
    }

    if (config.signer.signingMethod === 'mock' && process.env.NODE_ENV === 'production') {
        errors.push('Mock signing method cannot be used in production');
    }

    // Check for exposed private keys
    const exposedKeys = Object.entries(config.user.privateKeys).filter(
        ([name, key]) => key === process.env[`${name.toUpperCase()}_PRIVATE_KEY`]
    );

    if (exposedKeys.length > 0 && process.env.NODE_ENV === 'production') {
        warnings.push(`Private keys are hardcoded for: ${exposedKeys.map(([name]) => name).join(', ')}`);
    }

    if (errors.length > 0) {
        console.error('Configuration errors:', errors);
    }

    if (warnings.length > 0) {
        console.warn('Configuration warnings:', warnings);
    }

    return errors.length === 0;
}

// Auto-validate on import in development
if (process.env.NODE_ENV !== 'production') {
    validateConfig();
}
