import { RealmEdgeRpcProvider } from "./client";
import { ClientConfig } from "../provider";

/**
 * Example usage of the enhanced RealmEdgeRpcProvider
 */

// Example 1: Basic usage (backward compatible)
async function basicUsage() {
    console.log("=== Basic Usage (Backward Compatible) ===");

    const client = new RealmEdgeRpcProvider("http://localhost:8547");

    try {
        const result = await client.getLatestCheckpointTreeRoot();
        console.log("Latest checkpoint tree root:", result);
    } catch (error) {
        console.error("Error:", error);
    }
}

// Example 2: Enhanced usage with caching
async function cachingExample() {
    console.log("=== Caching Example ===");

    const config: ClientConfig = {
        cache: {
            ttl: 30000, // 30 seconds
            maxSize: 500,
            customTtl: new Map([
                ["qed_get_latest_checkpoint_tree_root", 10000], // 10 seconds
                ["qed_get_checkpoint_leaf_data", 60000], // 1 minute
            ]),
        },
    };

    const client = new RealmEdgeRpcProvider("http://localhost:8547", config);

    try {
        // First call - will hit the server
        console.log("First call (cache miss):");
        const result1 = await client.getCheckpointLeafData(1);
        console.log("Checkpoint data:", result1);

        // Second call - will use cache
        console.log("Second call (cache hit):");
        const result2 = await client.getCheckpointLeafData(1);
        console.log("Checkpoint data (cached):", result2);

        // Check cache stats
        console.log("Cache stats:", client.getCacheStats());
    } catch (error) {
        console.error("Error:", error);
    }
}

// Example 3: Retry logic
async function retryExample() {
    console.log("=== Retry Example ===");

    const config: ClientConfig = {
        retry: {
            maxAttempts: 5,
            baseDelay: 2000, // 2 seconds
            maxDelay: 30000, // 30 seconds max
            backoffMultiplier: 2,
            jitter: true,
        },
    };

    const client = new RealmEdgeRpcProvider("http://unreliable-server:8547", config);

    try {
        // This will retry up to 5 times with exponential backoff
        const result = await client.getLatestCheckpointTreeRoot();
        console.log("Result retrieved after retries:", result);
    } catch (error) {
        console.error("Failed after all retries:", error);
    }
}

// Example 4: Multi-provider with failover strategy
async function multiProviderFailoverExample() {
    console.log("=== Multi-Provider Failover Example ===");

    const urls = ["http://primary-realm:8547", "http://backup-realm:7547", "http://tertiary-realm:9547"];

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

    const client = new RealmEdgeRpcProvider(urls, config);

    try {
        // Will automatically failover to backup if primary fails
        const result = await client.getLatestCheckpointTreeRoot();
        console.log("Result from available provider:", result);

        // Check provider health
        console.log("Provider health:", client.getProviderHealth());
    } catch (error) {
        console.error("All providers failed:", error);
    }
}

// Example 5: Multi-provider with round-robin strategy
async function multiProviderRoundRobinExample() {
    console.log("=== Multi-Provider Round-Robin Example ===");

    const urls = ["http://realm-1:8547", "http://realm-2:8547", "http://realm-3:8547"];

    const config: ClientConfig = {
        multiProvider: {
            strategy: "round-robin",
            healthCheckInterval: 60000, // 1 minute
        },
        cache: {
            ttl: 15000, // 15 seconds
        },
    };

    const client = new RealmEdgeRpcProvider(urls, config);

    try {
        // Each call will go to the next provider in rotation
        for (let i = 0; i < 5; i++) {
            const result = await client.getLatestCheckpointTreeRoot();
            console.log(`Call ${i + 1} - Checkpoint tree root:`, result);
        }
    } catch (error) {
        console.error("Error:", error);
    }
}

// Example 6: Multi-provider with fastest strategy
async function multiProviderFastestExample() {
    console.log("=== Multi-Provider Fastest Example ===");

    const urls = ["http://realm-us:8547", "http://realm-eu:8547", "http://realm-asia:8547"];

    const config: ClientConfig = {
        multiProvider: {
            strategy: "fastest",
            healthCheckInterval: 45000, // 45 seconds
        },
    };

    const client = new RealmEdgeRpcProvider(urls, config);

    try {
        // Will automatically use the fastest responding provider
        const result = await client.getLatestCheckpointTreeRoot();
        console.log("Result from fastest provider:", result);

        // Check which provider was fastest
        const health = client.getProviderHealth();
        const fastest = health.reduce((prev, current) =>
            current.lastResponseTime < prev.lastResponseTime ? current : prev
        );
        console.log("Fastest provider:", fastest.url, "Response time:", fastest.lastResponseTime + "ms");
    } catch (error) {
        console.error("Error:", error);
    }
}

// Example 7: Multi-provider with parallel-first strategy
async function multiProviderParallelFirstExample() {
    console.log("=== Multi-Provider Parallel-First Example ===");

    const urls = ["http://realm-1:8547", "http://realm-2:8547"];

    const config: ClientConfig = {
        multiProvider: {
            strategy: "parallel-first",
            parallelRequestTimeout: 5000, // 5 seconds
        },
    };

    const client = new RealmEdgeRpcProvider(urls, config);

    try {
        // Will send requests to all providers and return the first successful response
        const result = await client.getLatestCheckpointTreeRoot();
        console.log("Result from first responding provider:", result);
    } catch (error) {
        console.error("Error:", error);
    }
}

// Example 8: Full configuration with all features
async function fullConfigurationExample() {
    console.log("=== Full Configuration Example ===");

    const urls = ["http://realm-primary:8547", "http://realm-secondary:8547"];

    const config: ClientConfig = {
        cache: {
            ttl: 60000, // 1 minute default
            maxSize: 1000,
            customTtl: new Map([
                // Fast-changing data
                ["qed_get_latest_checkpoint_tree_root", 5000],
                ["qed_get_latest_l2_block_state", 5000],

                // Slow-changing data
                ["qed_get_checkpoint_leaf_data", 300000], // 5 minutes
                ["qed_get_user_leaf_data", 300000],

                // Static data
                ["qed_get_user_registration_tree_root", 3600000], // 1 hour
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

    const client = new RealmEdgeRpcProvider(urls, config);

    try {
        // Example operations
        console.log("=== Production Example ===");

        // Get latest checkpoint tree root (cached for 5 seconds)
        const latestRoot = await client.getLatestCheckpointTreeRoot();
        console.log("Latest checkpoint tree root:", latestRoot);

        // Get checkpoint data (cached for 5 minutes)
        const checkpointData = await client.getCheckpointLeafData(1);
        console.log("Checkpoint data:", checkpointData);

        // Get user data (cached for 5 minutes)
        const userData = await client.getUserLeafData(1, 1);
        console.log("User data:", userData);

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

// Example 9: Error handling and monitoring
async function errorHandlingExample() {
    console.log("=== Error Handling Example ===");

    const urls = ["http://realm-1:8547", "http://realm-2:8547"];

    const config: ClientConfig = {
        multiProvider: {
            strategy: "failover",
            maxConsecutiveFailures: 2,
        },
        retry: {
            maxAttempts: 3,
        },
    };

    const client = new RealmEdgeRpcProvider(urls, config);

    try {
        const result = await client.getLatestCheckpointTreeRoot();
        console.log("Success:", result);
    } catch (error) {
        console.error("All providers failed:", error);

        // Check provider health for debugging
        const health = client.getProviderHealth();
        health.forEach((provider) => {
            console.log(`${provider.url}: ${provider.isHealthy ? "healthy" : "unhealthy"}`);
            console.log(`  Consecutive failures: ${provider.consecutiveFailures}`);
            console.log(`  Last response time: ${provider.lastResponseTime}ms`);
        });
    } finally {
        client.destroy();
    }
}

// Example 10: Performance comparison
async function performanceComparisonExample() {
    console.log("=== Performance Comparison Example ===");

    // Without caching
    const clientWithoutCache = new RealmEdgeRpcProvider("http://localhost:8547");

    console.time("Without cache - 5 calls");
    for (let i = 0; i < 5; i++) {
        await clientWithoutCache.getLatestCheckpointTreeRoot();
    }
    console.timeEnd("Without cache - 5 calls");

    // With caching
    const clientWithCache = new RealmEdgeRpcProvider("http://localhost:8547", {
        cache: { ttl: 60000 },
    });

    console.time("With cache - 5 calls");
    for (let i = 0; i < 5; i++) {
        await clientWithCache.getLatestCheckpointTreeRoot();
    }
    console.timeEnd("With cache - 5 calls");

    console.log("Cache stats:", clientWithCache.getCacheStats());

    // Clean up
    clientWithCache.destroy();
}

// Run all examples
async function runAllExamples() {
    console.log("🚀 Running RealmEdgeRpcProvider Examples\n");

    const examples = [
        { name: "Basic Usage", fn: basicUsage },
        { name: "Caching", fn: cachingExample },
        { name: "Retry Logic", fn: retryExample },
        { name: "Multi-Provider Failover", fn: multiProviderFailoverExample },
        { name: "Multi-Provider Round-Robin", fn: multiProviderRoundRobinExample },
        { name: "Multi-Provider Fastest", fn: multiProviderFastestExample },
        { name: "Multi-Provider Parallel-First", fn: multiProviderParallelFirstExample },
        { name: "Full Configuration", fn: fullConfigurationExample },
        { name: "Error Handling", fn: errorHandlingExample },
        { name: "Performance Comparison", fn: performanceComparisonExample },
    ];

    for (const example of examples) {
        try {
            console.log(`\n--- Running ${example.name} ---`);
            await example.fn();
        } catch (error) {
            console.error(`Error in ${example.name}:`, error);
        }
    }
}

// Export examples for individual testing
export {
    basicUsage,
    cachingExample,
    retryExample,
    multiProviderFailoverExample,
    multiProviderRoundRobinExample,
    multiProviderFastestExample,
    multiProviderParallelFirstExample,
    fullConfigurationExample,
    errorHandlingExample,
    performanceComparisonExample,
    runAllExamples,
};

// Note: To run examples directly, use: node -r ts-node/register examples.ts
