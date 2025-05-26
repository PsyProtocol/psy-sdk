import { ICityHTTPClient } from "../http/types";
import { FetchHTTPClient } from "../http/fetchClient";
import { ProofWithPublicInputs } from "../rpc/plonkTypes";
import {
    IRealmEdgeRpcProvider,
    MerkleProofCore,
    QEDCheckpointGlobalStateRoots,
    QEDCheckpointLeaf,
    QEDL2BlockState,
    QEDUserLeaf,
    QHashOut,
    RealmEdgeRPCCommand,
    SubmitUserEndCapNonProofInput,
} from "./types";

/**
 * Cache configuration interface
 */
interface CacheConfig {
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
interface RetryConfig {
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
interface MultiProviderConfig {
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
interface EnhancedClientConfig {
    cache?: CacheConfig;
    retry?: RetryConfig;
    multiProvider?: MultiProviderConfig;
}

/**
 * Cache entry interface
 */
interface CacheEntry<T> {
    value: T;
    timestamp: number;
    ttl: number;
}

/**
 * Provider health status interface
 */
interface ProviderHealth {
    url: string;
    isHealthy: boolean;
    consecutiveFailures: number;
    lastResponseTime: number;
    lastChecked: number;
}

/**
 * RealmEdgeRpcProvider implements the IRealmEdgeRpcProvider interface
 * with enhanced features including caching, retry logic, and multi-provider support.
 */
export class RealmEdgeRpcProvider implements IRealmEdgeRpcProvider {
    private httpClient: ICityHTTPClient;
    private urls: string[];
    private config: EnhancedClientConfig;

    // Cache system
    private cache = new Map<string, CacheEntry<any>>();
    private cacheConfig: Required<CacheConfig>;

    // Retry system
    private retryConfig: Required<RetryConfig>;

    // Multi-provider system
    private multiProviderConfig: Required<MultiProviderConfig>;
    private providerHealthMap = new Map<string, ProviderHealth>();
    private currentProviderIndex = 0;
    private healthCheckTimer?: ReturnType<typeof setInterval>;

    // Read-only methods that can be cached
    private readonly readOnlyMethods = new Set<string>([
        RealmEdgeRPCCommand.CheckUserIdInRealm,
        RealmEdgeRPCCommand.GetCheckpointLeafData,
        RealmEdgeRPCCommand.GetCheckpointLeafDataF,
        RealmEdgeRPCCommand.GetLatestL2BlockState,
        RealmEdgeRPCCommand.GetL2BlockState,
        RealmEdgeRPCCommand.GetL2BlockStateF,
        RealmEdgeRPCCommand.GetUserRegistrationTreeRoot,
        RealmEdgeRPCCommand.GetLatestCheckpointTreeRoot,
        RealmEdgeRPCCommand.GetCheckpointTreeRoot,
        RealmEdgeRPCCommand.GetCheckpointTreeRootF,
        RealmEdgeRPCCommand.GetCheckpointTreeLeafHash,
        RealmEdgeRPCCommand.GetCheckpointTreeLeafHashF,
        RealmEdgeRPCCommand.GetCheckpointTreeMerkleProof,
        RealmEdgeRPCCommand.GetCheckpointTreeMerkleProofF,
        RealmEdgeRPCCommand.GetCheckpointGlobalStateRoots,
        RealmEdgeRPCCommand.GetUserLeafData,
        RealmEdgeRPCCommand.GetUserLeafDataF,
        RealmEdgeRPCCommand.GetUserContractStateTreeRoot,
        RealmEdgeRPCCommand.GetUserContractStateTreeRootF,
        RealmEdgeRPCCommand.GetUserContractStateTreeLeafHash,
        RealmEdgeRPCCommand.GetUserContractStateTreeLeafHashF,
        RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProofF,
        RealmEdgeRPCCommand.GetUserContractTreeRoot,
        RealmEdgeRPCCommand.GetUserContractTreeRootF,
        RealmEdgeRPCCommand.GetUserContractTreeLeafHash,
        RealmEdgeRPCCommand.GetUserContractTreeLeafHashF,
        RealmEdgeRPCCommand.GetUserContractTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserContractTreeMerkleProofF,
        RealmEdgeRPCCommand.GetUserTreeRoot,
        RealmEdgeRPCCommand.GetUserTreeRootF,
        RealmEdgeRPCCommand.GetUserTreeLeafHash,
        RealmEdgeRPCCommand.GetUserTreeLeafHashF,
        RealmEdgeRPCCommand.GetUserBottomTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserBottomTreeMerkleProofF,
        RealmEdgeRPCCommand.GetUserSubTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserSubTreeMerkleProofF,
        RealmEdgeRPCCommand.GetUserTreeMerkleProof,
        RealmEdgeRPCCommand.GetUserTreeMerkleProofF,
    ]);

    constructor(
        urlOrUrls: string | string[],
        configOrHttpClient?: EnhancedClientConfig | ICityHTTPClient,
        httpClient?: ICityHTTPClient
    ) {
        // Parse constructor arguments for backward compatibility
        if (typeof urlOrUrls === "string") {
            this.urls = [urlOrUrls];

            if (configOrHttpClient && "sendRequest" in configOrHttpClient) {
                // Legacy usage: (url, httpClient)
                this.httpClient = configOrHttpClient;
                this.config = {};
            } else {
                // Enhanced usage with single URL: (url, config, httpClient?)
                this.config = configOrHttpClient || {};
                this.httpClient = httpClient || new FetchHTTPClient();
            }
        } else {
            // Enhanced usage with multiple URLs: (urls, config?, httpClient?)
            this.urls = urlOrUrls;
            this.config = (configOrHttpClient as EnhancedClientConfig) || {};
            this.httpClient = httpClient || new FetchHTTPClient();
        }

        // Initialize configurations with defaults
        this.cacheConfig = {
            ttl: this.config.cache?.ttl ?? 60000,
            maxSize: this.config.cache?.maxSize ?? 1000,
            enabledMethods: this.config.cache?.enabledMethods ?? this.readOnlyMethods,
            customTtl: this.config.cache?.customTtl ?? new Map(),
        };

        this.retryConfig = {
            maxAttempts: this.config.retry?.maxAttempts ?? 3,
            baseDelay: this.config.retry?.baseDelay ?? 1000,
            maxDelay: this.config.retry?.maxDelay ?? 30000,
            backoffMultiplier: this.config.retry?.backoffMultiplier ?? 2,
            retryableErrors: this.config.retry?.retryableErrors ?? [
                "ECONNREFUSED",
                "ENOTFOUND",
                "ECONNRESET",
                "ETIMEDOUT",
                "NetworkError",
                "TimeoutError",
                "AbortError",
            ],
            jitter: this.config.retry?.jitter ?? true,
        };

        this.multiProviderConfig = {
            strategy: this.config.multiProvider?.strategy ?? "failover",
            healthCheckInterval: this.config.multiProvider?.healthCheckInterval ?? 30000,
            healthCheckTimeout: this.config.multiProvider?.healthCheckTimeout ?? 5000,
            parallelRequestTimeout: this.config.multiProvider?.parallelRequestTimeout ?? 10000,
            maxConsecutiveFailures: this.config.multiProvider?.maxConsecutiveFailures ?? 3,
        };

        // Initialize provider health tracking
        this.initializeProviderHealth();

        // Start health checks if multiple providers are configured
        if (this.urls.length > 1 && this.config.multiProvider) {
            this.startHealthChecks();
        }
    }

    /**
     * Initialize provider health tracking
     */
    private initializeProviderHealth(): void {
        for (const url of this.urls) {
            this.providerHealthMap.set(url, {
                url,
                isHealthy: true,
                consecutiveFailures: 0,
                lastResponseTime: 0,
                lastChecked: 0,
            });
        }
    }

    /**
     * Start periodic health checks
     */
    private startHealthChecks(): void {
        this.healthCheckTimer = setInterval(() => {
            this.performHealthChecks();
        }, this.multiProviderConfig.healthCheckInterval);
    }

    /**
     * Perform health checks on all providers
     */
    private async performHealthChecks(): Promise<void> {
        const healthCheckPromises = this.urls.map(async (url) => {
            try {
                const startTime = Date.now();
                // Use the latest checkpoint method for health check
                await this.directRpc(url, RealmEdgeRPCCommand.GetLatestCheckpointTreeRoot as string, [], "1", "2.0");
                const responseTime = Date.now() - startTime;

                const health = this.providerHealthMap.get(url)!;
                health.isHealthy = true;
                health.consecutiveFailures = 0;
                health.lastResponseTime = responseTime;
                health.lastChecked = Date.now();
            } catch (error) {
                console.error(`Health check failed for ${url}:`, error);
                const health = this.providerHealthMap.get(url)!;
                health.consecutiveFailures++;
                health.lastChecked = Date.now();

                if (health.consecutiveFailures >= this.multiProviderConfig.maxConsecutiveFailures) {
                    health.isHealthy = false;
                }
            }
        });

        await Promise.allSettled(healthCheckPromises);
    }

    /**
     * Get healthy providers
     */
    private getHealthyProviders(): string[] {
        return this.urls.filter((url) => {
            const health = this.providerHealthMap.get(url);
            return health?.isHealthy ?? true;
        });
    }

    /**
     * Select next provider based on strategy
     */
    private selectProvider(): string {
        const healthyProviders = this.getHealthyProviders();

        if (healthyProviders.length === 0) {
            // Fallback to all providers if none are healthy
            return this.urls[0];
        }

        switch (this.multiProviderConfig.strategy) {
            case "round-robin":
                const provider = healthyProviders[this.currentProviderIndex % healthyProviders.length];
                this.currentProviderIndex++;
                return provider;

            case "fastest":
                return healthyProviders.reduce((fastest, current) => {
                    const fastestHealth = this.providerHealthMap.get(fastest)!;
                    const currentHealth = this.providerHealthMap.get(current)!;
                    return currentHealth.lastResponseTime < fastestHealth.lastResponseTime ? current : fastest;
                });

            case "failover":
            default:
                return healthyProviders[0];
        }
    }

    /**
     * Generate cache key for a request
     */
    private getCacheKey(method: string, params: any[]): string {
        return `${method}:${JSON.stringify(params)}`;
    }

    /**
     * Get value from cache
     */
    private getFromCache<T>(key: string): T | null {
        const entry = this.cache.get(key);
        if (!entry) return null;

        const now = Date.now();
        if (now - entry.timestamp > entry.ttl) {
            this.cache.delete(key);
            return null;
        }

        return entry.value;
    }

    /**
     * Set value in cache
     */
    private setCache<T>(key: string, value: T, method: string): void {
        if (this.cache.size >= this.cacheConfig.maxSize) {
            // Remove oldest entry (simple LRU)
            const firstKey = this.cache.keys().next().value;
            if (firstKey !== undefined) {
                this.cache.delete(firstKey);
            }
        }

        const ttl = this.cacheConfig.customTtl.get(method) ?? this.cacheConfig.ttl;
        this.cache.set(key, {
            value,
            timestamp: Date.now(),
            ttl,
        });
    }

    /**
     * Calculate retry delay with exponential backoff and jitter
     */
    private calculateRetryDelay(attempt: number): number {
        const exponentialDelay = Math.min(
            this.retryConfig.baseDelay * Math.pow(this.retryConfig.backoffMultiplier, attempt),
            this.retryConfig.maxDelay
        );

        if (!this.retryConfig.jitter) {
            return exponentialDelay;
        }

        // Add jitter (±25% of the delay)
        const jitter = exponentialDelay * 0.25 * (Math.random() * 2 - 1);
        return Math.max(0, exponentialDelay + jitter);
    }

    /**
     * Check if error is retryable
     */
    private isRetryableError(error: any): boolean {
        if (!error) return false;

        const errorMessage = error.message || error.toString();
        return this.retryConfig.retryableErrors.some((retryableError) => errorMessage.includes(retryableError));
    }

    /**
     * Sleep for specified milliseconds
     */
    private sleep(ms: number): Promise<void> {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }

    /**
     * Make a direct JSON-RPC request to a specific provider
     */
    private async directRpc<T>(url: string, method: string, params: any[], id = "1", jsonrpc = "2.0"): Promise<T> {
        const response = await this.httpClient.sendRequest({
            method: "POST",
            url,
            headers: {
                "Content-Type": "application/json",
            },
            body: JSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "json",
        });

        if (response.statusCode >= 400) {
            throw new Error(`RPC error: ${response.statusCode} - ${response.body}`);
        }

        const result = response.body;
        if (result.error) {
            throw new Error(`RPC error: ${result.error.message || JSON.stringify(result.error)}`);
        }

        return result.result as T;
    }

    /**
     * Make a JSON-RPC request with enhanced features
     */
    private async rpc<T>(method: string | RealmEdgeRPCCommand, params: any[], id = "1", jsonrpc = "2.0"): Promise<T> {
        const isReadOperation = this.readOnlyMethods.has(method);

        // Try cache first for read operations
        if (isReadOperation && this.config.cache && this.cacheConfig.enabledMethods.has(method)) {
            const cacheKey = this.getCacheKey(method, params);
            const cachedResult = this.getFromCache<T>(cacheKey);
            if (cachedResult !== null) {
                return cachedResult;
            }
        }

        let lastError: Error | null = null;

        // Retry logic
        for (let attempt = 0; attempt < this.retryConfig.maxAttempts; attempt++) {
            try {
                let result: T;

                if (this.urls.length === 1 || !this.config.multiProvider) {
                    // Single provider or multi-provider disabled
                    result = await this.directRpc<T>(this.urls[0], method, params, id, jsonrpc);
                } else {
                    // Multi-provider logic
                    switch (this.multiProviderConfig.strategy) {
                        case "parallel-first":
                            result = await this.executeParallelFirst<T>(method, params, id, jsonrpc);
                            break;
                        default:
                            result = await this.directRpc<T>(this.selectProvider(), method, params, id, jsonrpc);
                            break;
                    }
                }

                // Cache successful read operations
                if (isReadOperation && this.config.cache && this.cacheConfig.enabledMethods.has(method)) {
                    const cacheKey = this.getCacheKey(method, params);
                    this.setCache(cacheKey, result, method);
                }

                return result;
            } catch (error) {
                lastError = error as Error;

                // Update provider health on error
                if (this.urls.length > 1 && this.config.multiProvider) {
                    const currentUrl = this.selectProvider();
                    const health = this.providerHealthMap.get(currentUrl);
                    if (health) {
                        health.consecutiveFailures++;
                        if (health.consecutiveFailures >= this.multiProviderConfig.maxConsecutiveFailures) {
                            health.isHealthy = false;
                        }
                    }
                }

                // Check if we should retry
                if (attempt === this.retryConfig.maxAttempts - 1 || !this.isRetryableError(error)) {
                    break;
                }

                // Wait before retry
                const delay = this.calculateRetryDelay(attempt);
                await this.sleep(delay);
            }
        }

        throw lastError || new Error("Unknown error occurred");
    }

    /**
     * Execute parallel-first strategy
     */
    private async executeParallelFirst<T>(method: string, params: any[], id: string, jsonrpc: string): Promise<T> {
        const healthyProviders = this.getHealthyProviders();

        if (healthyProviders.length === 0) {
            throw new Error("No healthy providers available");
        }

        const promises = healthyProviders.map((url) => this.directRpc<T>(url, method, params, id, jsonrpc));

        try {
            return await Promise.race([
                Promise.race(promises),
                new Promise<never>((_, reject) =>
                    setTimeout(
                        () => reject(new Error("Parallel request timeout")),
                        this.multiProviderConfig.parallelRequestTimeout
                    )
                ),
            ]);
        } catch (error) {
            // If all parallel requests fail, try them sequentially
            for (const url of healthyProviders) {
                try {
                    return await this.directRpc<T>(url, method, params, id, jsonrpc);
                } catch (sequentialError) {
                    // Continue to next provider
                    console.error(`Sequential request failed: ${sequentialError}`);
                }
            }
            throw error;
        }
    }

    /**
     * Clear cache
     */
    public clearCache(): void {
        this.cache.clear();
    }

    /**
     * Get cache statistics
     */
    public getCacheStats(): { size: number; maxSize: number; hitRate?: number } {
        return {
            size: this.cache.size,
            maxSize: this.cacheConfig.maxSize,
        };
    }

    /**
     * Get provider health status
     */
    public getProviderHealth(): ProviderHealth[] {
        return Array.from(this.providerHealthMap.values());
    }

    /**
     * Cleanup resources
     */
    public destroy(): void {
        if (this.healthCheckTimer) {
            clearInterval(this.healthCheckTimer);
            this.healthCheckTimer = undefined;
        }
        this.clearCache();
    }

    // ========== RPC Interface Methods ==========

    // Check user ID in realm
    async checkUserIdInRealm(userId: bigint | number): Promise<boolean> {
        return this.rpc(RealmEdgeRPCCommand.CheckUserIdInRealm, [userId]);
    }

    // Submit user end cap
    async submitUserEndCap(userEcInput: SubmitUserEndCapNonProofInput, proof: ProofWithPublicInputs): Promise<string> {
        return this.rpc(RealmEdgeRPCCommand.SubmitUserEndCap, [userEcInput, proof]);
    }

    // Get checkpoint leaf data
    async getCheckpointLeafData(checkpointId: bigint | number): Promise<QEDCheckpointLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointLeafData, [checkpointId]);
    }

    async getCheckpointLeafDataF(checkpointId: bigint): Promise<QEDCheckpointLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointLeafDataF, [checkpointId]);
    }

    // Get L2 block state
    async getLatestL2BlockState(): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetLatestL2BlockState, []);
    }

    async getL2BlockState(checkpointId: bigint | number): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetL2BlockState, [checkpointId]);
    }

    async getL2BlockStateF(checkpointId: bigint): Promise<QEDL2BlockState> {
        return this.rpc(RealmEdgeRPCCommand.GetL2BlockStateF, [checkpointId]);
    }

    // Get user registration tree root
    async getUserRegistrationTreeRoot(checkpointId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserRegistrationTreeRoot, [checkpointId]);
    }

    // Get checkpoint tree roots
    async getLatestCheckpointTreeRoot(): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetLatestCheckpointTreeRoot, []);
    }

    async getCheckpointTreeRoot(checkpointId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeRoot, [checkpointId]);
    }

    async getCheckpointTreeRootF(checkpointId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeRootF, [checkpointId]);
    }

    // Get checkpoint tree leaf hash
    async getCheckpointTreeLeafHash(
        checkpointId: bigint | number,
        leafCheckpointId: bigint | number
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeLeafHash, [checkpointId, leafCheckpointId]);
    }

    async getCheckpointTreeLeafHashF(checkpointId: bigint, leafCheckpointId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeLeafHashF, [checkpointId, leafCheckpointId]);
    }

    // Get checkpoint tree merkle proof
    async getCheckpointTreeMerkleProof(
        checkpointId: bigint | number,
        leafCheckpointId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeMerkleProof, [checkpointId, leafCheckpointId]);
    }

    async getCheckpointTreeMerkleProofF(
        checkpointId: bigint,
        leafCheckpointId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointTreeMerkleProofF, [checkpointId, leafCheckpointId]);
    }

    // Get checkpoint global state roots
    async getCheckpointGlobalStateRoots(checkpointId: bigint | number): Promise<QEDCheckpointGlobalStateRoots> {
        return this.rpc(RealmEdgeRPCCommand.GetCheckpointGlobalStateRoots, [checkpointId]);
    }

    // Get user leaf data
    async getUserLeafData(checkpointId: bigint | number, userId: bigint | number): Promise<QEDUserLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetUserLeafData, [checkpointId, userId]);
    }

    async getUserLeafDataF(checkpointId: bigint, userId: bigint): Promise<QEDUserLeaf> {
        return this.rpc(RealmEdgeRPCCommand.GetUserLeafDataF, [checkpointId, userId]);
    }

    // Get user contract state tree root
    async getUserContractStateTreeRoot(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeRoot, [checkpointId, userId, contractId]);
    }

    async getUserContractStateTreeRootF(checkpointId: bigint, userId: bigint, contractId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeRootF, [checkpointId, userId, contractId]);
    }

    // Get user contract state tree leaf hash
    async getUserContractStateTreeLeafHash(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number,
        height: number,
        leafId: bigint | number
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeLeafHash, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    async getUserContractStateTreeLeafHashF(
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint,
        height: number,
        leafId: bigint
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeLeafHashF, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    // Get user contract state tree merkle proof
    async getUserContractStateTreeMerkleProof(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number,
        height: number,
        leafId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProof, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    async getUserContractStateTreeMerkleProofF(
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint,
        height: number,
        leafId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractStateTreeMerkleProofF, [
            checkpointId,
            userId,
            contractId,
            height,
            leafId,
        ]);
    }

    // Get user contract tree root
    async getUserContractTreeRoot(checkpointId: bigint | number, userId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeRoot, [checkpointId, userId]);
    }

    async getUserContractTreeRootF(checkpointId: bigint, userId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeRootF, [checkpointId, userId]);
    }

    // Get user contract tree leaf hash
    async getUserContractTreeLeafHash(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeLeafHash, [checkpointId, userId, contractId]);
    }

    async getUserContractTreeLeafHashF(checkpointId: bigint, userId: bigint, contractId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeLeafHashF, [checkpointId, userId, contractId]);
    }

    // Get user contract tree merkle proof
    async getUserContractTreeMerkleProof(
        checkpointId: bigint | number,
        userId: bigint | number,
        contractId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeMerkleProof, [checkpointId, userId, contractId]);
    }

    async getUserContractTreeMerkleProofF(
        checkpointId: bigint,
        userId: bigint,
        contractId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserContractTreeMerkleProofF, [checkpointId, userId, contractId]);
    }

    // Get user tree root
    async getUserTreeRoot(checkpointId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeRoot, [checkpointId]);
    }

    async getUserTreeRootF(checkpointId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeRootF, [checkpointId]);
    }

    // Get user tree leaf hash
    async getUserTreeLeafHash(checkpointId: bigint | number, userId: bigint | number): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeLeafHash, [checkpointId, userId]);
    }

    async getUserTreeLeafHashF(checkpointId: bigint, userId: bigint): Promise<QHashOut> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeLeafHashF, [checkpointId, userId]);
    }

    // Get user bottom tree merkle proof
    async getUserBottomTreeMerkleProof(
        rootLevel: number,
        checkpointId: bigint | number,
        userId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserBottomTreeMerkleProof, [rootLevel, checkpointId, userId]);
    }

    async getUserBottomTreeMerkleProofF(
        rootLevel: number,
        checkpointId: bigint,
        userId: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserBottomTreeMerkleProofF, [rootLevel, checkpointId, userId]);
    }

    // Get user sub tree merkle proof
    async getUserSubTreeMerkleProof(
        checkpointId: bigint | number,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserSubTreeMerkleProof, [checkpointId, rootLevel, leafLevel, leafIndex]);
    }

    async getUserSubTreeMerkleProofF(
        checkpointId: bigint,
        rootLevel: number,
        leafLevel: number,
        leafIndex: bigint
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserSubTreeMerkleProofF, [
            checkpointId,
            rootLevel,
            leafLevel,
            leafIndex,
        ]);
    }

    // Get user tree merkle proof
    async getUserTreeMerkleProof(
        checkpointId: bigint | number,
        userId: bigint | number
    ): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeMerkleProof, [checkpointId, userId]);
    }

    async getUserTreeMerkleProofF(checkpointId: bigint, userId: bigint): Promise<MerkleProofCore<QHashOut>> {
        return this.rpc(RealmEdgeRPCCommand.GetUserTreeMerkleProofF, [checkpointId, userId]);
    }
}
