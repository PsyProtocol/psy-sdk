# packages/contract-sdk/Makefile

# Directories
ABI_DIR = ./abi
GENERATED_DIR = ./generated
DEMO_DIR = ./demo

# Default target
default: help

# Show help
help:
	@echo "📚 Contract SDK Commands:"
	@echo "  make generate  - Generate SDK from default ABI"
	@echo "  make demo      - Run basic demo"
	@echo "  make clean     - Clean generated files"
	@echo "  make test      - Generate SDK and run demo"

# Generate SDK from default ABI
generate:
	@echo "🔧 Generating SDK from contract.abi.json..."
	@pnpm exec tsx src/cli.ts -i $(ABI_DIR)/contract.abi.json -o $(GENERATED_DIR)
	@echo "✅ SDK generated successfully!"

# Run demo
demo: generate
	@echo "🚀 Running basic demo..."
	@cd $(DEMO_DIR) && pnpm example:basic

# Clean generated files
clean:
	@echo "🧹 Cleaning generated files..."
	@rm -rf $(GENERATED_DIR)/*
	@touch $(GENERATED_DIR)/.gitkeep
	@rm -rf $(DEMO_DIR)/dist
	@echo "✅ Clean complete!"

# Full test cycle
test: clean generate demo
	@echo "✅ Test complete!"

# Development mode with file watching
watch:
	@echo "👀 Watching for changes..."
	@nodemon --watch $(ABI_DIR) --ext json --exec "make test"

.PHONY: default help generate demo clean test watch