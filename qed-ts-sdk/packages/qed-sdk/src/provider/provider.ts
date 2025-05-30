import { FetchHTTPClient, IHTTPClient } from "../http";
import { QedJSON } from "../utils";

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
export abstract class Provider {
    protected httpClient: IHTTPClient;
    protected urls: string[];
    protected config: ClientConfig;

    // Cache system
    private cache = new Map<string, CacheEntry<unknown>>();
    private cacheConfig: Required<CacheConfig>;

    // Retry system
    private retryConfig: Required<RetryConfig>;

    // Multi-provider system
    private multiProviderConfig: Required<MultiProviderConfig>;
    private providerHealthMap = new Map<string, ProviderHealth>();
    private currentProviderIndex = 0;
    private healthCheckTimer?: ReturnType<typeof setInterval>;

    // Abstract method to get read-only methods for caching
    protected abstract getReadOnlyMethods(): Set<string>;

    // Abstract method to get health check method
    protected abstract getHealthCheckMethod(): string;

    protected constructor(
        urlOrUrls: string | string[],
        configOrHttpClient?: ClientConfig | IHTTPClient,
        httpClient?: IHTTPClient
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
            this.config = (configOrHttpClient as ClientConfig) || {};
            this.httpClient = httpClient || new FetchHTTPClient();
        }

        // Initialize configurations with defaults
        this.cacheConfig = {
            ttl: this.config.cache?.ttl ?? 60000,
            maxSize: this.config.cache?.maxSize ?? 1000,
            enabledMethods: this.config.cache?.enabledMethods ?? this.getReadOnlyMethods(),
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
            this.performHealthChecks().then((r) => console.log(r));
        }, this.multiProviderConfig.healthCheckInterval);
    }

    /**
     * Perform health checks on all providers
     */
    private async performHealthChecks(): Promise<void> {
        const healthCheckPromises = this.urls.map(async (url) => {
            try {
                const startTime = Date.now();
                await this.directRpc(url, this.getHealthCheckMethod(), [], "1", "2.0");
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
                return healthyProviders[this.currentProviderIndex++ % healthyProviders.length];

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
    private getCacheKey(method: string, params: any): string {
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

        return entry.value as T;
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
    protected async directRpc<T>(url: string, method: string, params: any, id = "1", jsonrpc = "2.0"): Promise<T> {
        return this.directRpcWithHeaders<T>(url, method, params, id, jsonrpc);
    }

    /**
     * Execute parallel-first strategy
     */
    private async executeParallelFirst<T>(method: string, params: any, id: string, jsonrpc: string): Promise<T> {
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
     * Make a JSON-RPC request with enhanced features
     */
    protected async rpc<T>(
        method: string,
        params: unknown,
        id = "1",
        jsonrpc = "2.0",
        headers?: Record<string, string>
    ): Promise<T> {
        const isReadOperation = this.getReadOnlyMethods().has(method);

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
                    result = await this.directRpcWithHeaders<T>(this.urls[0], method, params, id, jsonrpc, headers);
                } else {
                    // Multi-provider logic
                    switch (this.multiProviderConfig.strategy) {
                        case "parallel-first":
                            result = await this.executeParallelFirst<T>(method, params, id, jsonrpc);
                            break;
                        default:
                            result = await this.directRpcWithHeaders<T>(
                                this.selectProvider(),
                                method,
                                params,
                                id,
                                jsonrpc,
                                headers
                            );
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
     * Make a direct JSON-RPC request with headers support
     */
    private async directRpcWithHeaders<T>(
        url: string,
        method: string,
        params: any,
        id = "1",
        jsonrpc = "2.0",
        customHeaders?: Record<string, string>
    ): Promise<T> {
        const headers: Record<string, string> = {
            "Content-Type": "application/json",
            ...customHeaders,
        };

        const response = await this.httpClient.sendRequest({
            method: "POST",
            url,
            headers,
            body: QedJSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "text",
        });

        if (response.statusCode >= 400) {
            throw new Error("Error in RPC call: " + QedJSON.stringify(response.body));
        }

        const result = QedJSON.parse(response.body);

        if (result.error) {
            throw new Error(`RPC error: ${result.error.message || JSON.stringify(result.error)}`);
        }

        return result.result as T;
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
}
