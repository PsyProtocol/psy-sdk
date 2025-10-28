import { CoordinatorEdgeRpcProvider } from "./client";
import { ClientConfig } from "../provider";

/**
 * Example 1: Basic usage (backward compatible)
 */
export async function basicUsageExample() {
    // Legacy usage - single URL, no enhanced features
    const client = new CoordinatorEdgeRpcProvider("http://localhost:8545");

    try {
        const checkpoint = await client.getLatestCheckpoint();
        console.log("Latest checkpoint:", checkpoint);
    } catch (error) {
        console.error("Error:", error);
    }
}

/**
 * Example 2: Enhanced usage with caching
 */
export async function cachingExample() {
    const config: ClientConfig = {
        cache: {
            ttl: 30000, // 30 seconds
            maxSize: 500,
            customTtl: new Map([
                ["qed_get_latest_checkpoint", 10000], // 10 seconds for latest checkpoint
                ["qed_get_checkpoint_leaf_data", 60000], // 1 minute for checkpoint data
            ]),
        },
    };

    const client = new CoordinatorEdgeRpcProvider("http://localhost:8545", config);

    try {
        // First call - will hit the server
        console.log("First call (cache miss):");
        const checkpoint1 = await client.getCheckpointLeafData(1);
        console.log("Checkpoint data:", checkpoint1);

        // Second call - will use cache
        console.log("Second call (cache hit):");
        const checkpoint2 = await client.getCheckpointLeafData(1);
        console.log("Checkpoint data (cached):", checkpoint2);

        // Check cache stats
        console.log("Cache stats:", client.getCacheStats());
    } catch (error) {
        console.error("Error:", error);
    }
}

/**
 * Example 3: Retry configuration
 */
export async function retryExample() {
    const config: ClientConfig = {
        retry: {
            maxAttempts: 5,
            baseDelay: 2000, // 2 seconds
            maxDelay: 30000, // 30 seconds max
            backoffMultiplier: 2,
            jitter: true,
        },
    };

    const client = new CoordinatorEdgeRpcProvider("http://unreliable-server:8545", config);

    try {
        // This will retry up to 5 times with exponential backoff
        const checkpoint = await client.getLatestCheckpoint();
        console.log("Checkpoint retrieved after retries:", checkpoint);
    } catch (error) {
        console.error("Failed after all retries:", error);
    }
}

/**
 * Example 4: Multi-provider with failover
 */
export async function multiProviderFailoverExample() {
    const urls = [
        "http://primary-coordinator:8545",
        "http://backup-coordinator:8545",
        "http://tertiary-coordinator:8545",
    ];

    const config: ClientConfig = {
        multiProvider: {
            strategy: "failover",
            healthCheckInterval: 30000, // 30 seconds
            maxConsecutiveFailures: 2,
        },
        retry: {
            maxAttempts: 2, // Reduce retries since we have multiple providers
        },
    };

    const client = new CoordinatorEdgeRpcProvider(urls, config);

    try {
        // Will automatically failover to backup if primary fails
        const checkpoint = await client.getLatestCheckpoint();
        console.log("Checkpoint from available provider:", checkpoint);

        // Check provider health
        console.log("Provider health:", client.getProviderHealth());
    } catch (error) {
        console.error("All providers failed:", error);
    }
}

/**
 * Example 5: Multi-provider with round-robin load balancing
 */
export async function multiProviderRoundRobinExample() {
    const urls = ["http://coordinator-1:8545", "http://coordinator-2:8545", "http://coordinator-3:8545"];

    const config: ClientConfig = {
        multiProvider: {
            strategy: "round-robin",
            healthCheckInterval: 60000, // 1 minute
        },
        cache: {
            ttl: 15000, // 15 seconds
        },
    };

    const client = new CoordinatorEdgeRpcProvider(urls, config);

    try {
        // Each call will go to the next provider in rotation
        for (let i = 0; i < 5; i++) {
            const checkpoint = await client.getLatestCheckpoint();
            console.log(`Call ${i + 1} - Checkpoint:`, checkpoint.checkpoint_id);
        }
    } catch (error) {
        console.error("Error:", error);
    }
}

/**
 * Example 6: Multi-provider with fastest response strategy
 */
export async function multiProviderFastestExample() {
    const urls = ["http://coordinator-us:8545", "http://coordinator-eu:8545", "http://coordinator-asia:8545"];

    const config: ClientConfig = {
        multiProvider: {
            strategy: "fastest",
            healthCheckInterval: 45000, // 45 seconds
        },
    };

    const client = new CoordinatorEdgeRpcProvider(urls, config);

    try {
        // Will automatically use the fastest responding provider
        const checkpoint = await client.getLatestCheckpoint();
        console.log("Checkpoint from fastest provider:", checkpoint);
    } catch (error) {
        console.error("Error:", error);
    }
}

/**
 * Example 7: Multi-provider with parallel-first strategy
 */
export async function multiProviderParallelExample() {
    const urls = ["http://coordinator-1:8545", "http://coordinator-2:8545"];

    const config: ClientConfig = {
        multiProvider: {
            strategy: "parallel-first",
            parallelRequestTimeout: 5000, // 5 seconds
        },
    };

    const client = new CoordinatorEdgeRpcProvider(urls, config);

    try {
        // Will send requests to all providers and return the first successful response
        const checkpoint = await client.getLatestCheckpoint();
        console.log("Checkpoint from first responding provider:", checkpoint);
    } catch (error) {
        console.error("Error:", error);
    }
}

/**
 * Example 8: Complete production configuration
 */
export async function productionConfigExample() {
    const urls = ["http://coordinator-primary:8545", "http://coordinator-secondary:8545"];

    const config: ClientConfig = {
        cache: {
            ttl: 60000, // 1 minute default
            maxSize: 1000,
            customTtl: new Map([
                // Fast-changing data
                ["qed_get_latest_checkpoint", 5000],
                ["qed_get_latest_l2_block_state", 5000],

                // Slow-changing data
                ["qed_get_checkpoint_leaf_data", 300000], // 5 minutes
                ["qed_get_contract_leaf_data", 300000],
                ["qed_get_user_registration_tree_root", 300000],

                // Static data
                ["qed_get_contract_code_definition", 3600000], // 1 hour
            ]),
        },
        retry: {
            maxAttempts: 3,
            baseDelay: 1000,
            maxDelay: 10000,
            backoffMultiplier: 2,
            jitter: true,
        },
        multiProvider: {
            strategy: "failover",
            healthCheckInterval: 30000,
            healthCheckTimeout: 5000,
            maxConsecutiveFailures: 3,
        },
    };

    const client = new CoordinatorEdgeRpcProvider(urls, config);

    try {
        // Example operations
        console.log("=== Production Example ===");

        // Get latest checkpoint (cached for 5 seconds)
        const latestCheckpoint = await client.getLatestCheckpoint();
        console.log("Latest checkpoint:", latestCheckpoint.checkpoint_id);

        // Get checkpoint data (cached for 5 minutes)
        const checkpointData = await client.getCheckpointLeafData(Number(latestCheckpoint.checkpoint_id));
        console.log("Checkpoint data:", checkpointData);

        // Get contract code (cached for 1 hour)
        const contractCode = await client.getContractCodeDefinition(1);
        console.log("Contract code:", contractCode);

        // Monitor system health
        console.log("Cache stats:", client.getCacheStats());
        console.log("Provider health:", client.getProviderHealth());
    } catch (error) {
        console.error("Production example error:", error);
    } finally {
        // Clean up resources
        client.destroy();
    }
}

/**
 * Example 10: Cache management
 */
export async function cacheManagementExample() {
    const config: ClientConfig = {
        cache: {
            ttl: 30000,
            maxSize: 100,
        },
    };

    const client = new CoordinatorEdgeRpcProvider("http://localhost:8545", config);

    try {
        // Fill cache with some data
        await client.getCheckpointLeafData(1);
        await client.getCheckpointLeafData(2);
        await client.getContractLeafData(1);

        console.log("Cache stats after operations:", client.getCacheStats());

        // Clear cache manually
        client.clearCache();
        console.log("Cache stats after clear:", client.getCacheStats());
    } catch (error) {
        console.error("Cache management error:", error);
    }
}

// Export all examples for easy testing
export const examples = {
    basicUsageExample,
    cachingExample,
    retryExample,
    multiProviderFailoverExample,
    multiProviderRoundRobinExample,
    multiProviderFastestExample,
    multiProviderParallelExample,
    productionConfigExample,
    cacheManagementExample,
};

// Run all examples (for testing purposes)
export async function runAllExamples() {
    console.log("Running all Coordinator RPC examples...");

    for (const [name, example] of Object.entries(examples)) {
        try {
            console.log(`\n--- Running ${name} ---`);
            await example();
        } catch (error) {
            console.error(`Error in ${name}:`, error);
        }
    }
}
