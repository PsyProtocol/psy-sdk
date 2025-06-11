# Changelog

All notable changes to the QED User Prover WASM project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial WebAssembly implementation of QED user prover
- Complete RPC interface compatibility with Rust local prover
- TypeScript integration with qed-ts-sdk
- Web Worker support for background processing
- Comprehensive test suite
- Performance benchmarking tools
- Documentation and examples

### Features

#### Core Functionality
- **Session Management**: Start and manage proving sessions
- **User Management**: Register, add, and switch between users
- **Local Proving**: Generate zero-knowledge proofs for contract calls
- **Cryptographic Operations**: ZK signatures, signature hashes, and key management
- **Contract Operations**: Deploy contracts and generate deployment commands
- **Batch Operations**: Prove multiple contract calls in a single operation

#### Integration
- **qed-ts-sdk Compatibility**: Drop-in replacement for RPC-based provider
- **TypeScript Support**: Full type definitions and type safety
- **Web Worker Integration**: Background processing to prevent UI blocking
- **Error Handling**: Comprehensive error types and handling
- **Memory Management**: Optimized for browser environments

#### Developer Experience
- **Build System**: Automated build with optimization
- **Testing**: Unit tests, integration tests, and performance tests
- **Documentation**: Comprehensive README and API documentation
- **Examples**: Usage examples for different scenarios
- **Linting**: ESLint and Prettier configuration

### Technical Details

#### Architecture
- **WASM Module**: Core proving logic compiled from Rust
- **TypeScript Wrapper**: High-level API compatible with qed-ts-sdk
- **Worker Proxy**: Web Worker integration for background processing
- **Type System**: Complete TypeScript definitions

#### Dependencies
- **Rust**: Core QED components, plonky2, wasm-bindgen
- **TypeScript**: qed-ts-sdk integration
- **Build Tools**: wasm-pack, wasm-opt, TypeScript compiler
- **Development**: ESLint, Prettier, testing frameworks

#### Performance
- **Optimized WASM**: Binary optimization with wasm-opt
- **Memory Efficient**: Careful memory management for browser environments
- **Background Processing**: Web Worker support for non-blocking operations
- **Lazy Loading**: Optional lazy loading of WASM module

### Browser Support
- Chrome/Edge 80+
- Firefox 79+
- Safari 14+
- Mobile browsers with WebAssembly support

### Security
- Private key handling within WASM module
- No sensitive data exposure to JavaScript
- Web Worker isolation for additional security
- Same cryptographic implementations as Rust version

## [0.1.0] - 2025-06-10

### Added
- Initial release of QED User Prover WASM
- Complete implementation of local prover RPC interface
- TypeScript integration with qed-ts-sdk
- Web Worker support
- Comprehensive documentation and examples

---

## Release Notes

### Version 0.1.0

This is the initial release of the QED User Prover WebAssembly module, providing a complete browser-based implementation of the QED zero-knowledge proof prover.

**Key Features:**
- Full RPC interface compatibility
- TypeScript integration
- Web Worker support
- Optimized for browser environments
- Comprehensive test suite

**Breaking Changes:**
- None (initial release)

**Migration Guide:**
- This is a new module, no migration needed
- Can be used as a drop-in replacement for RPC-based providers

**Known Issues:**
- Large proof generation may require significant memory
- Some advanced features may need additional optimization
- Browser compatibility limited to modern browsers with WebAssembly support

**Future Plans:**
- Performance optimizations
- Additional browser compatibility
- Enhanced error reporting
- More comprehensive examples
- Integration with additional QED components