import { FetchHTTPClient } from '../http/fetchClient.mjs';
import '../utils/felt.mjs';
import { PsyJSON } from '../utils/json.mjs';
import '../utils/random.mjs';

/**
 * Enhanced RPC Provider base class with caching, retry logic, and multi-provider support
 */
class Provider {
    constructor(urlOrUrls, configOrHttpClient, httpClient) {
        // Cache system
        this.cache = new Map();
        this.providerHealthMap = new Map();
        this.currentProviderIndex = 0;
        // Parse constructor arguments for backward compatibility
        if (typeof urlOrUrls === "string") {
            this.urls = [urlOrUrls];
            if (configOrHttpClient && "sendRequest" in configOrHttpClient) {
                // Legacy usage: (url, httpClient)
                this.httpClient = configOrHttpClient;
                this.config = {};
            }
            else {
                // Enhanced usage with single URL: (url, config, httpClient?)
                this.config = configOrHttpClient || {};
                this.httpClient = httpClient || new FetchHTTPClient();
            }
        }
        else {
            // Enhanced usage with multiple URLs: (urls, config?, httpClient?)
            this.urls = urlOrUrls;
            this.config = configOrHttpClient || {};
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
    initializeProviderHealth() {
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
    startHealthChecks() {
        this.healthCheckTimer = setInterval(() => {
            this.performHealthChecks().then((r) => console.log(r));
        }, this.multiProviderConfig.healthCheckInterval);
    }
    /**
     * Perform health checks on all providers
     */
    async performHealthChecks() {
        const healthCheckPromises = this.urls.map(async (url) => {
            try {
                const startTime = Date.now();
                await this.directRpc(url, this.getHealthCheckMethod(), [], "1", "2.0");
                const responseTime = Date.now() - startTime;
                const health = this.providerHealthMap.get(url);
                health.isHealthy = true;
                health.consecutiveFailures = 0;
                health.lastResponseTime = responseTime;
                health.lastChecked = Date.now();
            }
            catch (error) {
                console.error(`Health check failed for ${url}:`, error);
                const health = this.providerHealthMap.get(url);
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
    getHealthyProviders() {
        return this.urls.filter((url) => {
            const health = this.providerHealthMap.get(url);
            return health?.isHealthy ?? true;
        });
    }
    /**
     * Select next provider based on strategy
     */
    selectProvider() {
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
                    const fastestHealth = this.providerHealthMap.get(fastest);
                    const currentHealth = this.providerHealthMap.get(current);
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
    getCacheKey(method, params) {
        return `${method}:${PsyJSON.stringify(params)}`;
    }
    /**
     * Get value from cache
     */
    getFromCache(key) {
        const entry = this.cache.get(key);
        if (!entry)
            return null;
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
    setCache(key, value, method) {
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
    calculateRetryDelay(attempt) {
        const exponentialDelay = Math.min(this.retryConfig.baseDelay * Math.pow(this.retryConfig.backoffMultiplier, attempt), this.retryConfig.maxDelay);
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
    isRetryableError(error) {
        if (!error)
            return false;
        const errorMessage = error.message || error.toString();
        return this.retryConfig.retryableErrors.some((retryableError) => errorMessage.includes(retryableError));
    }
    /**
     * Sleep for specified milliseconds
     */
    sleep(ms) {
        return new Promise((resolve) => setTimeout(resolve, ms));
    }
    /**
     * Make a direct JSON-RPC request to a specific provider
     */
    async directRpc(url, method, params, id = "1", jsonrpc = "2.0") {
        return this.directRpcWithHeaders(url, method, params, id, jsonrpc);
    }
    /**
     * Execute parallel-first strategy
     */
    async executeParallelFirst(method, params, id, jsonrpc) {
        const healthyProviders = this.getHealthyProviders();
        if (healthyProviders.length === 0) {
            throw new Error("No healthy providers available");
        }
        const promises = healthyProviders.map((url) => this.directRpc(url, method, params, id, jsonrpc));
        try {
            return await Promise.race([
                Promise.race(promises),
                new Promise((_, reject) => setTimeout(() => reject(new Error("Parallel request timeout")), this.multiProviderConfig.parallelRequestTimeout)),
            ]);
        }
        catch (error) {
            // If all parallel requests fail, try them sequentially
            for (const url of healthyProviders) {
                try {
                    return await this.directRpc(url, method, params, id, jsonrpc);
                }
                catch (sequentialError) {
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
    async rpc(method, params, id = "1", jsonrpc = "2.0", headers) {
        // console.log(`RPC request to ${this.selectProvider()} with method ${method} and params ${params}`);
        return this.rpc_with_url(this.selectProvider(), method, params, id, jsonrpc, headers);
    }
    async rpc_with_url(url, method, params, id = "1", jsonrpc = "2.0", headers) {
        const isReadOperation = this.getReadOnlyMethods().has(method);
        // Try cache first for read operations
        if (isReadOperation && this.config.cache && this.cacheConfig.enabledMethods.has(method)) {
            const cacheKey = this.getCacheKey(method, params);
            const cachedResult = this.getFromCache(cacheKey);
            if (cachedResult !== null) {
                return cachedResult;
            }
        }
        let lastError = null;
        // Retry logic
        for (let attempt = 0; attempt < this.retryConfig.maxAttempts; attempt++) {
            try {
                let result;
                if (this.urls.length === 1 || !this.config.multiProvider) {
                    // Single provider or multi-provider disabled
                    result = await this.directRpcWithHeaders(this.urls[0], method, params, id, jsonrpc, headers);
                }
                else {
                    // Multi-provider logic
                    switch (this.multiProviderConfig.strategy) {
                        case "parallel-first":
                            result = await this.executeParallelFirst(method, params, id, jsonrpc);
                            break;
                        default:
                            result = await this.directRpcWithHeaders(url, method, params, id, jsonrpc, headers);
                            break;
                    }
                }
                // Cache successful read operations
                if (isReadOperation && this.config.cache && this.cacheConfig.enabledMethods.has(method)) {
                    const cacheKey = this.getCacheKey(method, params);
                    this.setCache(cacheKey, result, method);
                }
                return result;
            }
            catch (error) {
                lastError = error;
                // Update provider health on error
                if (this.urls.length > 1 && this.config.multiProvider) {
                    const currentUrl = url;
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
    async directRpcWithHeaders(url, method, params, id = "1", jsonrpc = "2.0", customHeaders) {
        const headers = {
            "Content-Type": "application/json",
            ...customHeaders,
        };
        const response = await this.httpClient.sendRequest({
            method: "POST",
            url,
            headers,
            body: PsyJSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "text",
        });
        if (response.statusCode >= 400) {
            throw new Error("Error in RPC call: " + PsyJSON.stringify(response.body));
        }
        const result = PsyJSON.parse(response.body);
        if (result.error) {
            throw new Error(`RPC error: ${result.error.message || PsyJSON.stringify(result.error)}`);
        }
        return result.result;
    }
    /**
     * Clear cache
     */
    clearCache() {
        this.cache.clear();
    }
    /**
     * Get cache statistics
     */
    getCacheStats() {
        return {
            size: this.cache.size,
            maxSize: this.cacheConfig.maxSize,
        };
    }
    /**
     * Get provider health status
     */
    getProviderHealth() {
        return Array.from(this.providerHealthMap.values());
    }
    /**
     * Cleanup resources
     */
    destroy() {
        if (this.healthCheckTimer) {
            clearInterval(this.healthCheckTimer);
            this.healthCheckTimer = undefined;
        }
        this.clearCache();
    }
}

export { Provider };
