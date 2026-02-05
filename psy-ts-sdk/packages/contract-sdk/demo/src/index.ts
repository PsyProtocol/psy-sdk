// Demo entry point
import { basicUsageExample } from "./examples/basic-usage";
async function main() {
    console.log("🚀 ZK Contract SDK Demo");
    console.log("======================\n");

    console.log("Available examples:");
    console.log("1. pnpm example:basic    - Basic usage demonstration");
    console.log("2. pnpm example:advanced - Advanced features");
    console.log("3. pnpm example:real     - Real-world DeFi dashboard\n");

    console.log("Running quick demonstration...\n");

    // Run basic example by default
    await basicUsageExample();

    console.log("\n✅ Demo complete! Try other examples to see more features.");
}

if (require.main === module) {
    main().catch(console.error);
}
