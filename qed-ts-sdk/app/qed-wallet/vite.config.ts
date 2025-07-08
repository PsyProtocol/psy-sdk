/* eslint-disable import/no-extraneous-dependencies */
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import topLevelAwait from "vite-plugin-top-level-await";
import wasm from "vite-plugin-wasm";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
    plugins: [
        react(),
        tsconfigPaths(),
        wasm(),
        topLevelAwait()
    ],
    // ... existing code ...
    optimizeDeps: {
        esbuildOptions: {
            // Node.js global to browser globalThis
            define: {
                global: "globalThis",
            },
        },
    },
    // Add WASM asset handling
    assetsInclude: ['**/local-web-prover/*.wasm'],
    server: {
        host: '0.0.0.0',
        port: 5173,
    },
});
