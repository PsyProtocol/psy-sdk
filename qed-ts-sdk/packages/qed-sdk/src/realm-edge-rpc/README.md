# Realm Edge RPC Client

The **RealmEdgeRpcProvider** is an enhanced RPC client for interacting with QED Realm Edge nodes. It extends the shared `Provider` base class and provides advanced features including caching, retry logic, multi-provider support, and automatic failover.

## Installation

```bash
npm install @qed/sdk
```

## Quick Start

### Basic Usage

```typescript
import { RealmEdgeRpcProvider } from "@qed/sdk";

// Simple usage
const client = new RealmEdgeRpcProvider("http://localhost:8545");

// Get latest checkpoint tree root
const root = await client.getLatestCheckpointTreeRoot();
console.log("Checkpoint tree root:", root);
```

### Enhanced Configuration

```typescript
import { RealmEdgeRpcProvider, ClientConfig } from "@qed/sdk";

const config: ClientConfig = {
    cache: {
        ttl: 60000, // 1 minute default cache
        maxSize: 1000,
        customTtl: new Map([
            ["qed_get_latest_checkpoint_tree_root", 5000], // 5 seconds
            ["qed_get_checkpoint_leaf_data", 300000], // 5 minutes
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
        maxConsecutiveFailures: 3,
    },
};

const client = new RealmEdgeRpcProvider(["http://realm-primary:8545", "http://realm-backup:8545"], config);
```

## Configuration Options

The `ClientConfig` interface (shared with other RPC clients) provides comprehensive configuration:

### Cache Configuration

```typescript
interface CacheConfig {
    ttl?: number; // Default TTL in milliseconds (60000)
    maxSize?: number; // Maximum cache entries (1000)
    enabledMethods?: Set<string>; // Methods to cache (all read methods)
    customTtl?: Map<string, number>; // Custom TTL per method
}
```

### Retry Configuration

```typescript
interface RetryConfig {
    maxAttempts?: number; // Maximum retry attempts (3)
    baseDelay?: number; // Base delay in milliseconds (1000)
    maxDelay?: number; // Maximum delay in milliseconds (30000)
    backoffMultiplier?: number; // Backoff multiplier (2)
    retryableErrors?: string[]; // Retryable error types
    jitter?: boolean; // Enable jitter (true)
}
```

### Multi-Provider Configuration

```typescript
interface MultiProviderConfig {
    strategy?: "failover" | "round-robin" | "fastest" | "parallel-first";
    healthCheckInterval?: number; // Health check interval (30000ms)
    healthCheckTimeout?: number; // Health check timeout (5000ms)
    parallelRequestTimeout?: number; // Parallel request timeout (10000ms)
    maxConsecutiveFailures?: number; // Max failures before unhealthy (3)
}
```

## API Methods

### User Operations

```typescript
// Check if user ID exists in realm
await client.checkUserIdInRealm(userId);

// Submit user end cap with proof
await client.submitUserEndCap(userEcInput, proof);

// Get user leaf data
await client.getUserLeafData(checkpointId, userId);
await client.getUserLeafDataF(checkpointId, userId); // Field version
```

### Checkpoint Operations

```typescript
// Get checkpoint data
await client.getCheckpointLeafData(checkpointId);
await client.getLatestCheckpointTreeRoot();
await client.getCheckpointTreeRoot(checkpointId);
await client.getCheckpointTreeMerkleProof(checkpointId, leafCheckpointId);
```

### L2 Block State

```typescript
// Get L2 block state
await client.getLatestL2BlockState();
await client.getL2BlockState(checkpointId);
await client.getL2BlockStateF(checkpointId); // Field version
```

### User Tree Operations

```typescript
// Get user tree data
await client.getUserTreeRoot(checkpointId);
await client.getUserTreeLeafHash(checkpointId, userId);
await client.getUserTreeMerkleProof(checkpointId, userId);

// Get user bottom tree merkle proof
await client.getUserBottomTreeMerkleProof(rootLevel, checkpointId, userId);

// Get user sub tree merkle proof
await client.getUserSubTreeMerkleProof(checkpointId, rootLevel, leafLevel, leafIndex);
```

### User Contract Operations

```typescript
// Get user contract state tree
await client.getUserContractStateTreeRoot(checkpointId, userId, contractId);
await client.getUserContractStateTreeLeafHash(checkpointId, userId, contractId, height, leafId);
await client.getUserContractStateTreeMerkleProof(checkpointId, userId, contractId, height, leafId);

// Get user contract tree
await client.getUserContractTreeRoot(checkpointId, userId);
await client.getUserContractTreeLeafHash(checkpointId, userId, contractId);
await client.getUserContractTreeMerkleProof(checkpointId, userId, contractId);
```

## Load Balancing Strategies

### Failover Strategy

Routes requests to the primary provider, automatically failing over to backup providers when the primary becomes unhealthy.

```typescript
const client = new RealmEdgeRpcProvider(["http://realm-primary:8545", "http://realm-backup:8545"], {
    multiProvider: { strategy: "failover" },
});
```

### Round-Robin Strategy

Distributes requests evenly across all healthy providers.

```typescript
const client = new RealmEdgeRpcProvider(["http://realm-1:8545", "http://realm-2:8545", "http://realm-3:8545"], {
    multiProvider: { strategy: "round-robin" },
});
```

### Fastest Strategy

Routes requests to the provider with the lowest response time.

```typescript
const client = new RealmEdgeRpcProvider(["http://realm-us:8545", "http://realm-eu:8545", "http://realm-asia:8545"], {
    multiProvider: { strategy: "fastest" },
});
```

### Parallel-First Strategy

Sends requests to all providers simultaneously and returns the first successful response.

```typescript
const client = new RealmEdgeRpcProvider(["http://realm-1:8545", "http://realm-2:8545"], {
    multiProvider: { strategy: "parallel-first" },
});
```

## Monitoring and Debugging

### Cache Statistics

```typescript
const stats = client.getCacheStats();
console.log("Cache size:", stats.size);
console.log("Max size:", stats.maxSize);
console.log("Hit rate:", stats.hitRate);
```

### Provider Health

```typescript
const health = client.getProviderHealth();
health.forEach((provider) => {
    console.log(`${provider.url}: ${provider.isHealthy ? "healthy" : "unhealthy"}`);
    console.log(`Response time: ${provider.lastResponseTime}ms`);
    console.log(`Consecutive failures: ${provider.consecutiveFailures}`);
});
```

### Clear Cache

```typescript
client.clearCache();
```

### Cleanup Resources

```typescript
client.destroy(); // Stops health checks and clears cache
```

## Error Handling

The client automatically handles various error scenarios:

- **Network Errors**: Automatic retry with exponential backoff
- **Provider Failures**: Automatic failover to healthy providers
- **Timeout Errors**: Configurable timeouts with retry logic
- **Rate Limiting**: Intelligent backoff strategies

```typescript
try {
    const result = await client.getLatestCheckpointTreeRoot();
    console.log("Success:", result);
} catch (error) {
    console.error("All providers failed:", error);

    // Check provider health for debugging
    const health = client.getProviderHealth();
    health.forEach((provider) => {
        console.log(`${provider.url}: ${provider.isHealthy ? "healthy" : "unhealthy"}`);
    });
}
```

## Performance Optimization

### Caching Best Practices

```typescript
const config: ClientConfig = {
    cache: {
        ttl: 60000, // Default 1 minute
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
};
```

### Multi-Provider Optimization

```typescript
const config: ClientConfig = {
    multiProvider: {
        strategy: "fastest", // Use fastest responding provider
        healthCheckInterval: 30000, // Check health every 30 seconds
        maxConsecutiveFailures: 2, // Mark unhealthy after 2 failures
    },
    retry: {
        maxAttempts: 2, // Reduce retries since we have multiple providers
    },
};
```

## Migration from Legacy Client

The new RealmEdgeRpcProvider is backward compatible with existing code:

```typescript
// Legacy usage (still works)
const client = new RealmEdgeRpcProvider("http://localhost:8545");

// Enhanced usage (recommended)
const client = new RealmEdgeRpcProvider("http://localhost:8545", {
    cache: { ttl: 60000 },
    retry: { maxAttempts: 3 },
});
```

## Examples

See the [examples file](./examples.ts) for comprehensive usage examples including:

- Basic usage patterns
- Caching strategies
- Multi-provider configurations
- Error handling
- Performance optimization
- Production configurations

## Type Definitions

All types are exported from the main package:

```typescript
import {
    RealmEdgeRpcProvider,
    ClientConfig,
    IRealmEdgeRpcProvider,
    RealmEdgeRPCCommand,
    QEDUserLeaf,
    QEDCheckpointLeaf,
    QEDL2BlockState,
    MerkleProofCore,
    QHashOut,
} from "@qed/sdk";
```

## Architecture Benefits

By extending the shared `Provider` base class, RealmEdgeRpcProvider:

- **Reduces Code Duplication**: Shares common functionality with other RPC clients
- **Ensures Consistency**: Uses the same configuration interfaces and patterns
- **Simplifies Maintenance**: Bug fixes and improvements benefit all clients
- **Improves Testing**: Shared test utilities and patterns
- **Enhances Documentation**: Consistent API documentation across clients

This architecture follows SOLID principles and promotes code reusability while maintaining the specific functionality required for Realm Edge operations.

## License

MIT License - see LICENSE file for details.
