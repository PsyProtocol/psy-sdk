/**
 * Example 1: Basic usage (backward compatible)
 */
export declare function basicUsageExample(): Promise<void>;
/**
 * Example 2: Enhanced usage with caching
 */
export declare function cachingExample(): Promise<void>;
/**
 * Example 3: Retry configuration
 */
export declare function retryExample(): Promise<void>;
/**
 * Example 4: Multi-provider with failover
 */
export declare function multiProviderFailoverExample(): Promise<void>;
/**
 * Example 5: Multi-provider with round-robin load balancing
 */
export declare function multiProviderRoundRobinExample(): Promise<void>;
/**
 * Example 6: Multi-provider with fastest response strategy
 */
export declare function multiProviderFastestExample(): Promise<void>;
/**
 * Example 7: Multi-provider with parallel-first strategy
 */
export declare function multiProviderParallelExample(): Promise<void>;
/**
 * Example 8: Complete production configuration
 */
export declare function productionConfigExample(): Promise<void>;
/**
 * Example 10: Cache management
 */
export declare function cacheManagementExample(): Promise<void>;
export declare const examples: {
    basicUsageExample: typeof basicUsageExample;
    cachingExample: typeof cachingExample;
    retryExample: typeof retryExample;
    multiProviderFailoverExample: typeof multiProviderFailoverExample;
    multiProviderRoundRobinExample: typeof multiProviderRoundRobinExample;
    multiProviderFastestExample: typeof multiProviderFastestExample;
    multiProviderParallelExample: typeof multiProviderParallelExample;
    productionConfigExample: typeof productionConfigExample;
    cacheManagementExample: typeof cacheManagementExample;
};
export declare function runAllExamples(): Promise<void>;
//# sourceMappingURL=examples.d.ts.map