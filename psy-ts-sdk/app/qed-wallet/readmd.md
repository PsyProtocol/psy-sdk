# app wallet

## extension wallet

## web wallet

### build web wallet

#### build wasm binary

```bash
cd qedlang-rust && make wasm-build
```

#### build psy-ts-sdk

```bash
cd psy-ts-sdk && pnpm install && pnpm build
```

#### build web wallet

```bash
cd app && pnpm install && pnpm build
```

### run web wallet

```bash
cd qedlang-rust && make run-all
```

### test web wallet

#### open browser

```bash
http://localhost:5173
```

or

```bash
http://localhost:5174
```

#### deploy contract

```bash
cd qedlang-rust && ./scripts/run_scenario0.sh
```

#### import private key

```bash
17c975c2668ebe0ca7c87f67c6414ebb7fd664f46370a0af2a3b204c8824ac5a
```

* build block

```bash
make build-block
make build-block
make build-block
```

#### call contract

```bash
Contract ID: 0
Method Name: simple_mint
Parameters: 1000
```

### debug web wallet

F12 to open dev tools, select console tab to see the log.

### Q&A

1. user operation need to make a block(make build-block),wait for the block to be committed, then can do the next operation.
2. debug web wallet, F12 to open dev tools, select console tab to see the log.

### How to use web wallet?

1. Open browser and go to http://localhost:5173 or http://localhost:5174
2. Click "import wallet" button to import the private key
3. Click "transfer" button to call the contract