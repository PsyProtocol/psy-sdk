// Type declaration for WASM module imports
declare module '*.wasm' {
  const wasmModule: WebAssembly.Module;
  export default wasmModule;
}

// Alternative: if you want to import as binary data
declare module '*.wasm?inline' {
  const wasmBinary: Uint8Array;
  export default wasmBinary;
}

// Type declaration for WASM init imports
declare module '*.wasm?init' {
  const wasmInit: () => Promise<WebAssembly.Instance>;
  export default wasmInit;
}