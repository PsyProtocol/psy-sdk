// packages/codegen/src/generator/index.ts

import { readFileSync, writeFileSync, mkdirSync, existsSync } from "fs";
import { join } from "path";
import { OriginalFormatContractGenerator } from "./original-format-generator";
import { TypesGenerator } from "./types-generator";
import { DecoderGenerator } from "./decoder-generator";
import { AbiConverter } from "../converters/abi-converter";
import { AbiInput, isAbi } from "../types/abi-format";
import { PsyJSON } from "@psy-protocol/psy-sdk";

export class SDKGenerator {
    constructor(private outputDir: string) {}

    async generateFromAbiFile(abiPath: string): Promise<void> {
        if (!existsSync(abiPath)) {
            throw new Error(`ABI file not found: ${abiPath}`);
        }

        console.log(`📖 Reading ABI from ${abiPath}`);
        const abiContent = readFileSync(abiPath, "utf-8");
        const abiData = PsyJSON.parse(abiContent) as AbiInput;

        // Validate ABI shape and generate.
        if (!isAbi(abiData)) {
            throw new Error('Invalid ABI: must contain "contract" and "schema_version"');
        }
        console.log("📋 Processing ABI");
        await this.generateFromAbi(abiData);
    }

    // Convert the ABI to the internal representation, then generate the SDK files.
    private async generateFromAbi(abi: AbiInput): Promise<void> {
        console.log("🔄 Converting ABI to internal representation...");

        // Convert to internal format for generation.
        const converter = new AbiConverter();
        const internalFormat = converter.convert(abi);

        console.log(`✅ Processed ${internalFormat.contracts.length} contracts`);

        // Generate SDK files
        await this.generateSDK(internalFormat);
    }

    private async generateSDK(abi: any): Promise<void> {
        mkdirSync(this.outputDir, { recursive: true });

        const files = new Map<string, string>();

        // Generate decoder
        console.log("📝 Generating decoder...");
        const decoderGenerator = new DecoderGenerator();
        files.set("decoder.ts", decoderGenerator.generate());

        // Generate types
        console.log("📝 Generating types...");
        const typesGenerator = new TypesGenerator();
        files.set("types.ts", typesGenerator.generate());

        // Generate contracts
        console.log("📝 Generating contracts...");
        for (const contract of abi.contracts) {
            const contractGenerator = new OriginalFormatContractGenerator(contract);
            const contractCode = contractGenerator.generate();
            files.set(`${contract.name}.ts`, contractCode);
            console.log(`  ✓ Generated ${contract.name}.ts`);
        }

        // Generate index
        const contractNames = abi.contracts.map((c: any) => c.name);
        files.set("index.ts", this.generateIndex(contractNames));

        // Generate README
        files.set("README.md", this.generateReadme(contractNames));

        // Write all files
        for (const [filename, content] of files) {
            this.writeFile(filename, content);
        }
    }

    private generateIndex(contractNames: string[]): string {
        const contractImports = contractNames.map((name) => `export { ${name} } from './${name}';`).join("\n");

        return `// Auto-generated index file - Do not edit manually
export * from './types';
export { RecursiveDecoder } from './decoder';
${contractImports}

// Re-export common types for convenience
export type { Felt, IContractStateReader, ISigner } from './types';
export { Signer } from './types';
`;
    }

    private generateReadme(contractNames: string[]): string {
        const contractList = contractNames.map((c) => `- \`${c}\``).join("\n");
        const firstContract = contractNames[0] || "Contract";

        return `# Generated ZK Contract SDK

This SDK was auto-generated from the contract ABI.

## Contracts

${contractList}

## Usage

### Basic Usage (Read-Only)

\`\`\`typescript
import { ${firstContract} } from './index';

// Initialize provider
const provider = {
  async getContractState(contractId, userId, slots) {
    // Your implementation
  },
  async sendTransaction(contractId, functionName, args, publicKey?) {
    // Your implementation
  }
};

// Create contract instance for reading
const contract = new ${firstContract}(userId, contractId, provider);

// Read state variables
const balance = await contract.balance;
\`\`\`

### With Signer (Read + Write)

\`\`\`typescript
import { ${firstContract}, Signer } from './index';

// Create a signer with your public key
const publicKey = "0x...";
const signer = Signer.fromPublicKey(publicKey, provider);

// Create contract with signer
const contract = new ${firstContract}(userId, contractId, signer);

// Now you can call state-changing functions
await contract.simple_mint(1000n);
\`\`\`
`;
    }

    private writeFile(filename: string, content: string): void {
        const filepath = join(this.outputDir, filename);
        writeFileSync(filepath, content, "utf-8");
        console.log(`  📄 Written: ${filename}`);
    }
}
