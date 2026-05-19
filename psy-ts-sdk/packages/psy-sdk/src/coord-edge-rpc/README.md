# Coordinator Edge RPC Provider

The Coordinator Edge RPC Provider is a powerful TypeScript client for interacting with Psy Coordinator nodes. It extends the basic RPC functionality with advanced features including caching, retry logic, multi-provider support, and load balancing.

## Installation

```bash
npm install @psy-protocol/psy-sdk
```

## Quick Start

### Basic Usage (Backward Compatible)

```typescript
import { CoordinatorEdgeRpcProvider } from "@psy-protocol/psy-sdk";

// Simple usage - no enhanced features
const client = new CoordinatorEdgeRpcProvider("http://localhost:8545");

const checkpoint = await client.getLatestCheckpoint();
console.log("Latest checkpoint:", checkpoint);
```

### Enhanced Usage with Caching

```typescript
import { CoordinatorEdgeRpcProvider, EnhancedClientConfig } from "@psy-protocol/psy-sdk";

const config: EnhancedClientConfig = {
    cache: {
        ttl: 60000, // 1 minute default cache
        maxSize: 1000,
        customTtl: new Map([
            ["psy_get_latest_checkpoint", 10000], // 10 seconds
            ["psy_get_checkpoint_leaf_data", 300000], // 5 minutes
        ]),
    },
};

const client = new CoordinatorEdgeRpcProvider("http://localhost:8545", config);

// First call hits the server, second call uses cache
const data1 = await client.getCheckpointLeafData(1);
const data2 = await client.getCheckpointLeafData(1); // Cached!
```

### Multi-Provider with Failover

```typescript
const urls = ["http://primary-coordinator:8545", "http://backup-coordinator:8545", "http://tertiary-coordinator:8545"];

const config: EnhancedClientConfig = {
    multiProvider: {
        strategy: "failover",
        healthCheckInterval: 30000,
        maxConsecutiveFailures: 3,
    },
    retry: {
        maxAttempts: 3,
        baseDelay: 1000,
        backoffMultiplier: 2,
    },
};

const client = new CoordinatorEdgeRpcProvider(urls, config);

// Automatically fails over to backup if primary is down
const checkpoint = await client.getLatestCheckpoint();
```

## Configuration Options

### Cache Configuration

```typescript
interface CacheConfig {
    ttl?: number; // Default TTL in milliseconds (60000)
    maxSize?: number; // Maximum cache entries (1000)
    enabledMethods?: Set<string>; // Methods to cache (all read methods)
    customTtl?: Map<string, number>; // Method-specific TTL settings
}
```

### Retry Configuration

```typescript
interface RetryConfig {
    maxAttempts?: number; // Maximum retry attempts (3)
    baseDelay?: number; // Base delay in milliseconds (1000)
    maxDelay?: number; // Maximum delay in milliseconds (30000)
    backoffMultiplier?: number; // Exponential backoff multiplier (2)
    retryableErrors?: string[]; // Error types to retry
    jitter?: boolean; // Add jitter to prevent thundering herd (true)
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

## API Reference

### Core Methods

#### User Management

```typescript
// Register a new user
await client.registerUser(zkPublicKey);

// Get user ID from QHash
const userId = await client.getUserId(qhash);
```

#### Contract Operations

```typescript
// Deploy a contract
await client.deployContract(contractData);

// Get contract information
const contract = await client.getContractLeafData(contractId);
const code = await client.getContractCodeDefinition(contractId);
```

#### Checkpoint Operations

```typescript
// Get latest checkpoint
const latest = await client.getLatestCheckpoint();

// Get checkpoint data
const checkpoint = await client.getCheckpointLeafData(checkpointId);

// Get checkpoint sync info
const syncInfo = await client.getCheckpointSyncInfo(checkpointId);
```

#### Tree Operations

```typescript
// User tree operations
const userRoot = await client.getUserTreeRoot(checkpointId);
const userProof = await client.getUserTreeMerkleProof(checkpointId, userId);

// Contract tree operations
const contractRoot = await client.getContractTreeRoot(checkpointId);
const contractProof = await client.getContractTreeMerkleProof(checkpointId, contractId);

// Checkpoint tree operations
const checkpointRoot = await client.getCheckpointTreeRoot(checkpointId);
const checkpointProof = await client.getCheckpointTreeMerkleProof(checkpointId, leafId);
```

### Enhanced Methods

#### Cache Management

```typescript
// Get cache statistics
const stats = client.getCacheStats();

// Clear cache manually
client.clearCache();
```

#### Provider Health Monitoring

```typescript
// Get provider health status
const health = client.getProviderHealth();
console.log("Provider health:", health);
```

#### Resource Cleanup

```typescript
// Clean up resources (timers, cache, etc.)
client.destroy();
```

## Production Configuration Example

```typescript
const productionConfig: EnhancedClientConfig = {
    cache: {
        ttl: 60000, // 1 minute default
        maxSize: 1000,
        customTtl: new Map([
            // Fast-changing data
            ["psy_get_latest_checkpoint", 5000],
            ["psy_get_latest_block_state", 5000],

            // Slow-changing data
            ["psy_get_checkpoint_leaf_data", 300000], // 5 minutes
            ["psy_get_contract_leaf_data", 300000],

            // Static data
            ["psy_get_contract_code_definition", 3600000], // 1 hour
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

const client = new CoordinatorEdgeRpcProvider(
    ["http://coordinator-primary:8545", "http://coordinator-secondary:8545"],
    productionConfig
);
```

## Error Handling

The client automatically handles various error scenarios:

- **Network Errors**: Automatic retry with exponential backoff
- **Provider Failures**: Automatic failover to healthy providers
- **Timeout Errors**: Configurable timeouts with retry logic
- **Authentication Errors**: Proper JWT token handling

```typescript
try {
    const result = await client.getLatestCheckpoint();
} catch (error) {
    if (error.message.includes("All providers failed")) {
        // Handle complete system failure
    } else if (error.message.includes("Authentication")) {
        // Handle auth errors
    } else {
        // Handle other errors
    }
}
```

## Performance Considerations

### Caching Strategy

- **Read-heavy operations**: Enable caching with appropriate TTL
- **Real-time data**: Use shorter TTL or disable caching
- **Static data**: Use longer TTL (hours/days)

### Multi-Provider Strategy

- **High availability**: Use 'failover' strategy
- **Load distribution**: Use 'round-robin' strategy
- **Low latency**: Use 'fastest' strategy
- **Maximum speed**: Use 'parallel-first' strategy

### Memory Usage

- Monitor cache size with `getCacheStats()`
- Adjust `maxSize` based on available memory
- Call `destroy()` to clean up resources

## Best Practices

1. **Always call `destroy()`** when done to clean up resources
2. **Use appropriate cache TTL** based on data volatility
3. **Monitor provider health** in production environments
4. **Configure retry logic** based on network reliability

## Examples

See the [examples file](./examples.ts) for comprehensive usage examples including:

- Basic usage patterns
- Caching configurations
- Multi-provider setups
- Production configurations
- Error handling strategies

## Support

For issues and questions:

- Check the [examples](./examples.ts) for common patterns
- Review the [type definitions](./types.ts) for API details
- Consult the main Psy documentation
