# QED User Prover WebAssembly Module

This module provides a complete WebAssembly implementation of the QED user-side zero-knowledge proof prover, enabling browser-based proving capabilities with full RPC interface compatibility.

## Features

- **Complete RPC Interface**: Implements all methods from the Rust local prover
- **TypeScript Integration**: Seamless integration with `qed-ts-sdk`
- **Web Worker Support**: Background processing to prevent UI blocking
- **Memory Efficient**: Optimized for browser environments
- **Cross-Platform**: Works in all modern browsers
- **Type Safety**: Full TypeScript definitions included

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Browser Environment                      │
├─────────────────────────────────────────────────────────────┤
│  TypeScript Application                                     │
│  ┌─────────────────┐    ┌─────────────────────────────────┐ │
│  │ qed-ts-sdk      │    │ QED WASM Provider               │ │
│  │                 │◄──►│ - Direct Mode                   │ │
│  │ IQEDUserProver  │    │ - Worker Mode                   │ │
│  │ Provider        │    │ - Type Conversions              │ │
│  └─────────────────┘    └─────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│  Web Worker (Optional)                                      │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ QED WASM Worker                                         │ │
│  │ - Background Processing                                 │ │
│  │ - Message Passing                                       │ │
│  │ - Resource Management                                   │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│  WebAssembly Module                                         │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │ QED User Prover WASM                                    │ │
│  │ - Core Proving Logic                                    │ │
│  │ - Cryptographic Operations                              │ │
│  │ - Session Management                                    │ │
│  │ - User Management                                       │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### 1. Build the WASM Module

```bash
# Install wasm-pack if not already installed
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build the WASM module
./build.sh
```

### 2. Install TypeScript Dependencies

```bash
cd integration
npm install
npm run build
```

### 3. Basic Usage

```typescript
import { createQEDWasmProvider } from './integration/qed-wasm-provider';

// Initialize the provider
const provider = await createQEDWasmProvider({
  debug: true,
  useWorker: false, // Set to true for background processing
});

// Start a session
const sessionId = await provider.startSession();

// Generate a keypair
const keypair = await provider.getRandomKeypair();

// Register and add user
const userId = 'user_' + Date.now();
await provider.registerUser(userId, keypair.publicKey);
await provider.addUser(userId, keypair.privateKey);
await provider.switchUser(userId);

// Prove a contract call
const proof = await provider.proveContractCall(
  '0x1234567890abcdef', // contract address
  'add', // function name
  ['10', '20'], // arguments
  circuitDefinition // circuit definition
);

// Cleanup
await provider.dispose();
```

### 4. Web Worker Usage

```typescript
import { QEDWasmWorkerProvider } from './integration/qed-wasm-worker-proxy';

// Create worker provider
const workerProvider = new QEDWasmWorkerProvider('./qed-wasm-worker.js');

// Initialize
await workerProvider.initialize();

// Use the same API as direct provider
const sessionId = await workerProvider.startSession();

// Cleanup
await workerProvider.dispose();
```

## API Reference

The WASM provider implements the complete `IQEDUserProverProvider` interface:

### Session Management
- `ping(): Promise<string>` - Test connectivity
- `startSession(): Promise<string>` - Start a new session

### User Management
- `registerUser(userId: string, publicKey: string): Promise<boolean>`
- `addUser(userId: string, privateKey: string): Promise<boolean>`
- `switchUser(userId: string): Promise<boolean>`
- `getZKPublicKey(): Promise<string>`
- `getRandomKeypair(): Promise<{publicKey: string, privateKey: string}>`

### Proving Operations
- `proveContractCall(...)` - Prove a single contract call
- `proveContractCalls(...)` - Prove multiple contract calls
- `signAndSubmit(...)` - Sign and submit a proof

### Contract Operations
- `deployContract(...)` - Deploy a contract
- `getDeployContractCmd(...)` - Get deployment command

### Cryptographic Operations
- `getSigHash(message: string): Promise<string>`
- `getZKSignature(message: string): Promise<string>`
- `getEndCapProof(...): Promise<ProofWithPublicInputs>`
- `getUserECInput(userId: string): Promise<string>`

### Utility
- `getResult(resultId: string): Promise<string>`

## Configuration Options

```typescript
interface QEDWasmProviderConfig {
  /** Path to the WASM binary file */
  wasmPath?: string;
  /** Enable debug logging */
  debug?: boolean;
  /** Use Web Worker for background processing */
  useWorker?: boolean;
  /** Memory limit for WASM module (in MB) */
  memoryLimit?: number;
}
```

## Performance Considerations

### Memory Usage
- The WASM module is optimized for browser environments
- Memory usage scales with proof complexity
- Use Web Workers for memory-intensive operations

### Computation Time
- Proof generation can take several seconds for complex circuits
- Use Web Workers to prevent UI blocking
- Consider showing progress indicators for long operations

### File Size
- The WASM binary is optimized with `wasm-opt`
- Gzip compression is recommended for serving
- Consider lazy loading for better initial page load

## Browser Compatibility

- **Chrome/Edge**: Full support (v80+)
- **Firefox**: Full support (v79+)
- **Safari**: Full support (v14+)
- **Mobile**: iOS Safari 14+, Chrome Mobile 80+

### Required Features
- WebAssembly
- Web Workers (for background processing)
- SharedArrayBuffer (for advanced features)
- BigInt support

## Integration with qed-ts-sdk

The WASM provider is designed to be a drop-in replacement for the RPC-based provider:

```typescript
// Before (RPC-based)
import { QEDRPCUserProverProvider } from 'qed-ts-sdk';
const provider = new QEDRPCUserProverProvider('http://localhost:8080');

// After (WASM-based)
import { createQEDWasmProvider } from './qed-wasm-provider';
const provider = await createQEDWasmProvider();

// Same API, no code changes needed!
const proof = await provider.proveContractCall(...);
```

## Error Handling

The WASM provider includes comprehensive error handling:

```typescript
try {
  const proof = await provider.proveContractCall(...);
} catch (error) {
  if (error.message.includes('WASM')) {
    // WASM-specific error
    console.error('WASM error:', error);
  } else if (error.message.includes('Worker')) {
    // Worker-related error
    console.error('Worker error:', error);
  } else {
    // General proving error
    console.error('Proving error:', error);
  }
}
```

## Security Considerations

- Private keys are handled securely within the WASM module
- No sensitive data is exposed to JavaScript
- Web Workers provide additional isolation
- All cryptographic operations use the same secure implementations as the Rust version

## Development

### Building from Source

```bash
# Install Rust and wasm-pack
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build the project
cargo build --release
./build.sh

# Build TypeScript integration
cd integration
npm install
npm run build
```

### Testing

```bash
# Run examples
cd integration
npm test

# Or run specific examples
node dist/example.js
```

### Debugging

Enable debug mode for detailed logging:

```typescript
const provider = await createQEDWasmProvider({
  debug: true,
});
```

## Troubleshooting

### Common Issues

1. **WASM module fails to load**
   - Check that the WASM file is served with correct MIME type
   - Ensure CORS headers are set if loading from different origin
   - Verify browser supports WebAssembly

2. **Worker initialization fails**
   - Check that worker script is accessible
   - Verify SharedArrayBuffer is available (requires HTTPS)
   - Check browser console for detailed error messages

3. **Memory errors**
   - Reduce memory limit in configuration
   - Use Web Workers for memory-intensive operations
   - Consider breaking large operations into smaller chunks

4. **Performance issues**
   - Enable Web Workers for background processing
   - Use `wasm-opt` for binary optimization
   - Consider preloading the WASM module

### Getting Help

- Check the [examples](./integration/example.ts) for usage patterns
- Review browser console for error messages
- Ensure all dependencies are properly installed
- Verify WASM module was built successfully

## License

This project is licensed under the MIT License - see the LICENSE file for details.