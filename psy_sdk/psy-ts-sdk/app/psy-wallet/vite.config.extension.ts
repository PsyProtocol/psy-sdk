/* eslint-disable import/no-extraneous-dependencies */
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tsconfigPaths from "vite-tsconfig-paths";
import { resolve } from "path";
import fs from "fs";

export default defineConfig({
    css: {
        preprocessorOptions: {
            scss: {
                api: 'modern'
            }
        }
    },
    plugins: [
        react(), 
        tsconfigPaths(),
        {
            name: 'copy-manifest',
            writeBundle() {
                fs.copyFileSync('public/manifest.json', 'dist-extension/manifest.json');
                
                // Copy icons
                if (fs.existsSync('public/icon-16.png')) {
                    fs.copyFileSync('public/icon-16.png', 'dist-extension/icon-16.png');
                }
                if (fs.existsSync('public/icon-48.png')) {
                    fs.copyFileSync('public/icon-48.png', 'dist-extension/icon-48.png');
                }
                if (fs.existsSync('public/icon-128.png')) {
                    fs.copyFileSync('public/icon-128.png', 'dist-extension/icon-128.png');
                }
                
                // Copy favicon
                if (fs.existsSync('public/psy-favicon.ico')) {
                    fs.copyFileSync('public/psy-favicon.ico', 'dist-extension/psy-favicon.ico');
                }
                
                // Copy other files
                if (fs.existsSync('public/buffer.min.js')) {
                    fs.copyFileSync('public/buffer.min.js', 'dist-extension/buffer.min.js');
                }
            }
        }
    ],
    optimizeDeps: {
        esbuildOptions: {
            // Node.js global to browser globalThis
            define: {
                global: "globalThis",
            },
        },
    },
    build: {
        outDir: "dist-extension",
        rollupOptions: {
            input: {
                popup: resolve(__dirname, "index.html"),
                approve: resolve(__dirname, "src/components/DappService/index.html"),
                background: resolve(__dirname, "src/background/index.js"), 
                contentScript: resolve(__dirname, "src/contentScript/index.js"), 
                webHook: resolve(__dirname, "src/webHook/index.js")
            },
            output: {
                entryFileNames: "assets/[name].js",
                chunkFileNames: "assets/[name].js",
                assetFileNames: "assets/[name].[ext]",
                manualChunks: {
                    vendor: ['react', 'react-dom', 'react-router-dom'],
                    mantine: ['@mantine/core', '@mantine/hooks'],
                    styles: ['styled-components']
                }
            },
        },
        // Disable code splitting for extension
        cssCodeSplit: false,
        // Set a reasonable chunk size limit
        chunkSizeWarningLimit: 3000,
        // Enable minification using esbuild (default)
        minify: 'esbuild'
    },
    // Ensure assets are loaded with relative paths
    base: "./",
});