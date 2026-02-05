#!/usr/bin/env node
// packages/contract-sdk/src/cli.ts

import { program } from "commander";
import { SDKGenerator } from "./generator";
import { resolve } from "path";

program
    .name("contract-sdk-gen")
    .description("Generate TypeScript SDK from ZK contract ABI")
    .version("1.0.0")
    .requiredOption("-i, --input <path>", "Input ABI JSON file path")
    .option("-o, --output <path>", "Output directory", "./generated")
    .parse(process.argv);

const options = program.opts();

async function main() {
    try {
        console.log("🚀 Starting SDK generation...");

        const inputPath = resolve(options.input);
        const outputPath = resolve(options.output);

        const generator = new SDKGenerator(outputPath);
        await generator.generateFromAbiFile(inputPath);

        console.log("✨ SDK generation complete!");
        console.log(`📁 Output directory: ${outputPath}`);
    } catch (error) {
        console.error("❌ Error generating SDK:", error);
        process.exit(1);
    }
}

main();
