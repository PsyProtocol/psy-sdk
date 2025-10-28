/**
 * Jest setup file for Psy SDK tests
 */
import { beforeAll } from "@jest/globals";

// Global test configuration
beforeAll(() => {
    // Log test environment info
    console.log("🧪 Psy SDK Test Environment Setup");
    console.log("📡 RPC URL:", process.env.Psy_RPC_URL || "http://localhost:8545");
    console.log("🌍 Node Environment:", process.env.NODE_ENV || "test");
    console.log("⏱️  Default Timeout: 30000ms");
});

// Global error handler for unhandled promise rejections
process.on("unhandledRejection", (reason, promise) => {
    console.error("Unhandled Rejection at:", promise, "reason:", reason);
    // It's good practice to understand why rejections are unhandled.
    // In a test environment, you might want to fail the test:
    // throw reason;
});
