# QED User Prover WASM Module

This module provides WASM bindings for the QED User Prover, enabling zero-knowledge proof generation in web environments.

## Features

- **RPC Server Implementation**: Complete WASM-compatible wrapper for the RPC server
- **User Management**: Register, add, and switch between users
- **Contract Operations**: Deploy contracts and execute contract calls
- **Proof Generation**: Generate ZK proofs and signatures
- **Session Management**: Manage proving sessions and store results

## WASM Exports

### WasmRpcServer

The main WASM-compatible RPC server implementation.

#### Constructor

```javascript
const server = new WasmRpcServer(rpcConfigJson);
```

- `rpcConfigJson`: JSON string containing RPC configuration

#### Methods

##### Session Management

- `start_session()`: Start a new proving session
- `ping(message)`: Ping the server with a message

##### User Operations

- `register_user(privateKeyStr)`: Register a new user
- `add_user(privateKeyStr)`: Add an existing user
- `switch_user(publicKeyHashStr)`: Switch to a different user
- `get_zk_public_key_json(privateKeyStr)`: Get ZK public key info
- `get_random_keypair_json()`: Generate a random keypair

##### Contract Operations

- `deploy_contract_json(circuitDefsJson)`: Deploy a contract
- `get_deploy_contract_cmd_json(circuitDefsJson)`: Get deployment command
- `prove_contract_call_json(contractCallJson)`: Prove a single contract call
- `prove_contract_calls_json(contractCallsJson)`: Prove multiple contract calls

##### Signature Operations

- `get_sighash(networkMagic)`: Get signature hash
- `get_zk_signature_json(sighashStr)`: Generate ZK signature
- `get_end_cap_proof_json(signatureProofJson)`: Get end cap proof
- `get_user_ec_input_json()`: Get user end cap input

##### Result Management

- `sign_and_submit()`: Sign and submit transaction
- `get_result(idStr)`: Get result by ID

## Usage Example

```javascript
// Initialize the WASM module
import init, { WasmRpcServer } from "./pkg/qed_user_prover.js";

async function main() {
    // Initialize WASM
    await init();

    // Create RPC configuration
    const rpcConfig = {
        rpc_url: "http://localhost:8080",
        network: "testnet",
        // ... other config fields
    };

    // Create RPC server
    const server = new WasmRpcServer(JSON.stringify(rpcConfig));

    // Start a session
    const sessionResult = server.start_session();
    console.log("Session started:", sessionResult);

    // Generate a random keypair
    const keypair = JSON.parse(server.get_random_keypair_json());
    console.log("Generated keypair:", keypair);

    // Register the user
    const publicKey = server.register_user(keypair.private_key);
    console.log("User registered with public key:", publicKey);

    // Test ping
    const pingResult = server.ping("Hello WASM!");
    console.log("Ping result:", pingResult); // Should return "!MSAW olleH"
}

main().catch(console.error);
```

## Build Instructions

### Prerequisites

- Rust toolchain with `wasm32-unknown-unknown` target
- `wasm-pack` tool

### Building

```bash
# Install wasm-pack if not already installed
cargo install wasm-pack

# Build the WASM module
wasm-pack build --target web --out-dir pkg

# Or build for Node.js
wasm-pack build --target nodejs --out-dir pkg-node
```

### Build Outputs

The build will generate:

- `pkg/qed_user_prover.js` - JavaScript bindings
- `pkg/qed_user_prover_bg.wasm` - WebAssembly binary
- `pkg/qed_user_prover.d.ts` - TypeScript definitions
- `pkg/package.json` - NPM package metadata

## Integration

### Web Browser

```html
<!DOCTYPE html>
<html>
    <head>
        <script type="module">
            import init, { WasmRpcServer } from "./pkg/qed_user_prover.js";

            async function run() {
                await init();
                // Use the WASM module...
            }

            run();
        </script>
    </head>
    <body>
        <!-- Your web app content -->
    </body>
</html>
```

### Node.js

```javascript
const { WasmRpcServer } = require("./pkg-node/qed_user_prover.js");

// Use the modules...
```

## Error Handling

All WASM methods return `Result` types that must be handled:

```javascript
try {
    const result = server.start_session();
    console.log("Success:", result);
} catch (error) {
    console.error("Error:", error.message);
}
```

## Type Definitions

The module includes TypeScript definitions for better development experience:

```typescript
import { WasmRpcServer } from "./pkg/qed_user_prover";

const server: WasmRpcServer = new WasmRpcServer(configJson);
const result: string = server.start_session();
```

## Performance Considerations

- WASM modules are optimized for size and performance
- Consider using Web Workers for CPU-intensive operations
- Cache compiled WASM modules when possible
- Use streaming compilation for large WASM files

## Troubleshooting

### Common Issues

1. **Module initialization fails**: Ensure WASM is properly initialized with `await init()`
2. **JSON parsing errors**: Validate JSON strings before passing to methods
3. **Memory issues**: Monitor memory usage for long-running applications

### Debug Build

For debugging, build with debug symbols:

```bash
wasm-pack build --target web --dev --out-dir pkg-debug
```

## Contributing

1. Ensure all WASM methods handle errors gracefully
2. Add comprehensive tests for new functionality
3. Update documentation for API changes
4. Test in multiple browser environments
