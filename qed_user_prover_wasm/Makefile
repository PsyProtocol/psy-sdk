.PHONY: all build test clean install deps check format lint example help

# Default target
all: build

# Build the WASM module and TypeScript integration
build:
	@echo "🔨 Building QED User Prover WASM..."
	./build.sh
	@echo "📦 Building TypeScript integration..."
	cd integration && npm run build
	@echo "✅ Build completed!"

# Install all dependencies
install: deps

deps:
	@echo "📥 Installing dependencies..."
	@echo "Checking for wasm-pack..."
	@which wasm-pack > /dev/null || (echo "Installing wasm-pack..." && curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh)
	@echo "Installing Node.js dependencies..."
	cd integration && npm install
	@echo "✅ Dependencies installed!"

# Run tests
test: build
	@echo "🧪 Running tests..."
	cd integration && npm test
	@echo "✅ Tests completed!"

# Run examples
example: build
	@echo "🚀 Running examples..."
	cd integration && npm run example

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -rf pkg/
	cd integration && npm run clean
	@echo "✅ Clean completed!"

# Format code
format:
	@echo "🎨 Formatting code..."
	cargo fmt
	cd integration && npm run format
	@echo "✅ Formatting completed!"

# Lint code
lint:
	@echo "🔍 Linting code..."
	cargo clippy -- -D warnings
	cd integration && npm run lint
	@echo "✅ Linting completed!"

# Check code quality
check: format lint
	@echo "✅ Code quality check completed!"

# Development setup
dev-setup: deps
	@echo "🛠️  Setting up development environment..."
	rustup component add clippy rustfmt
	@echo "✅ Development setup completed!"

# Quick development build (no optimization)
dev-build:
	@echo "🔨 Building for development..."
	wasm-pack build --target web --out-dir pkg --dev
	cd integration && npm run build
	@echo "✅ Development build completed!"

# Release build with optimizations
release-build:
	@echo "🔨 Building for release..."
	./build.sh
	cd integration && npm run build
	@echo "✅ Release build completed!"

# Benchmark tests
benchmark: build
	@echo "⚡ Running benchmarks..."
	cd integration && node -e "require('./dist/test.js').QEDWasmTest.prototype.runPerformanceTest()"
	@echo "✅ Benchmarks completed!"

# Size analysis
size-analysis: build
	@echo "📊 Analyzing build sizes..."
	@echo "WASM binary size:"
	@ls -lh pkg/*.wasm | awk '{print $$5 " " $$9}'
	@echo "\nJavaScript bundle size:"
	@ls -lh pkg/*.js | awk '{print $$5 " " $$9}'
	@echo "\nTypeScript bundle size:"
	@ls -lh integration/dist/*.js | awk '{print $$5 " " $$9}' || echo "No TypeScript bundles found"

# Documentation
docs:
	@echo "📚 Generating documentation..."
	cargo doc --no-deps
	@echo "✅ Documentation generated!"
	@echo "Open target/doc/qed_user_prover_wasm/index.html to view Rust docs"

# Help
help:
	@echo "QED User Prover WASM - Available commands:"
	@echo ""
	@echo "  make build         - Build WASM module and TypeScript integration"
	@echo "  make install       - Install all dependencies"
	@echo "  make test          - Run all tests"
	@echo "  make example       - Run usage examples"
	@echo "  make clean         - Clean build artifacts"
	@echo "  make format        - Format all code"
	@echo "  make lint          - Lint all code"
	@echo "  make check         - Run format and lint checks"
	@echo "  make dev-setup     - Set up development environment"
	@echo "  make dev-build     - Quick development build"
	@echo "  make release-build - Optimized release build"
	@echo "  make benchmark     - Run performance benchmarks"
	@echo "  make size-analysis - Analyze build sizes"
	@echo "  make docs          - Generate documentation"
	@echo "  make help          - Show this help message"
	@echo ""
	@echo "For more information, see README.md"