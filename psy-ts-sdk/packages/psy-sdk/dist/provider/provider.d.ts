import { IHTTPClient } from "../http";
/**
 * Cache configuration interface
 */
export interface CacheConfig {
    /** Cache TTL in milliseconds (default: 60000) */
    ttl?: number;
    /** Maximum cache size (default: 1000) */
    maxSize?: number;
    /** Methods to enable caching for (default: all read methods) */
    enabledMethods?: Set<string>;
    /** Custom TTL for specific methods */
    customTtl?: Map<string, number>;
}
/**
 * Retry configuration interface
 */
export interface RetryConfig {
    /** Maximum retry attempts (default: 3) */
    maxAttempts?: number;
    /** Base delay in milliseconds (default: 1000) */
    baseDelay?: number;
    /** Maximum delay in milliseconds (default: 30000) */
    maxDelay?: number;
    /** Backoff multiplier (default: 2) */
    backoffMultiplier?: number;
    /** Retryable error types (default: network and timeout errors) */
    retryableErrors?: string[];
    /** Enable jitter to prevent thundering herd (default: true) */
    jitter?: boolean;
}
/**
 * Multi-provider configuration interface
 */
export interface MultiProviderConfig {
    /** Load balancing strategy (default: 'failover') */
    strategy?: "failover" | "round-robin" | "fastest" | "parallel-first";
    /** Health check interval in milliseconds (default: 30000) */
    healthCheckInterval?: number;
    /** Health check timeout in milliseconds (default: 5000) */
    healthCheckTimeout?: number;
    /** Parallel request timeout in milliseconds (default: 10000) */
    parallelRequestTimeout?: number;
    /** Max consecutive failures before marking unhealthy (default: 3) */
    maxConsecutiveFailures?: number;
}
/**
 * Enhanced client configuration interface
 */
export interface ClientConfig {
    cache?: CacheConfig;
    retry?: RetryConfig;
    multiProvider?: MultiProviderConfig;
}
/**
 * Provider health status interface
 */
export interface ProviderHealth {
    url: string;
    isHealthy: boolean;
    consecutiveFailures: number;
    lastResponseTime: number;
    lastChecked: number;
}
/**
 * Enhanced RPC Provider base class with caching, retry logic, and multi-provider support
 */
export declare abstract class Provider {
    protected httpClient: IHTTPClient;
    protected urls: string[];
    protected config: ClientConfig;
    private cache;
    private cacheConfig;
    private retryConfig;
    private multiProviderConfig;
    private providerHealthMap;
    private currentProviderIndex;
    private healthCheckTimer?;
    protected abstract getReadOnlyMethods(): Set<string>;
    protected abstract getHealthCheckMethod(): string;
    protected constructor(urlOrUrls: string | string[], configOrHttpClient?: ClientConfig | IHTTPClient, httpClient?: IHTTPClient);
    /**
     * Initialize provider health tracking
     */
    private initializeProviderHealth;
    /**
     * Start periodic health checks
     */
    private startHealthChecks;
    /**
     * Perform health checks on all providers
     */
    private performHealthChecks;
    /**
     * Get healthy providers
     */
    private getHealthyProviders;
    /**
     * Select next provider based on strategy
     */
    private selectProvider;
    /**
     * Generate cache key for a request
     */
    private getCacheKey;
    /**
     * Get value from cache
     */
    private getFromCache;
    /**
     * Set value in cache
     */
    private setCache;
    /**
     * Calculate retry delay with exponential backoff and jitter
     */
    private calculateRetryDelay;
    /**
     * Check if error is retryable
     */
    private isRetryableError;
    /**
     * Sleep for specified milliseconds
     */
    private sleep;
    /**
     * Make a direct JSON-RPC request to a specific provider
     */
    protected directRpc<T>(url: string, method: string, params: any, id?: string, jsonrpc?: string): Promise<T>;
    /**
     * Execute parallel-first strategy
     */
    private executeParallelFirst;
    /**
     * Make a JSON-RPC request with enhanced features
     */
    protected rpc<T>(method: string, params: unknown, id?: string, jsonrpc?: string, headers?: Record<string, string>): Promise<T>;
    protected rpc_with_url<T>(url: string, method: string, params: unknown, id?: string, jsonrpc?: string, headers?: Record<string, string>): Promise<T>;
    /**
     * Make a direct JSON-RPC request with headers support
     */
    private directRpcWithHeaders;
    /**
     * Clear cache
     */
    clearCache(): void;
    /**
     * Get cache statistics
     */
    getCacheStats(): {
        size: number;
        maxSize: number;
        hitRate?: number;
    };
    /**
     * Get provider health status
     */
    getProviderHealth(): ProviderHealth[];
    /**
     * Cleanup resources
     */
    destroy(): void;
}
//# sourceMappingURL=provider.d.ts.map