import { AbiConverter } from "../converters/abi-converter";
import { AbiFormat, FieldPath, InternalContract, InternalFunction, InternalStruct } from "../types/abi-format";
import { RecursiveDecoder } from "./decoder";
import { createMerkleHelper } from "./merkle-helper";
import { createVariableProxy, IFlatVariablePosition } from "./proxy";
import { Felt, IContractProvider, ISigner } from "./types";
import "./types"; // side-effect: patches Array.prototype.toFelts

export interface ContractOptions {
    checkpointId: Felt;
    userId: Felt;
}

function isViewFunction(fn: InternalFunction): boolean {
    return (
        fn.name.startsWith("get_") ||
        fn.name.startsWith("view_") ||
        (fn.return_size > 0 && !fn.name.includes("mint") && !fn.name.includes("transfer") && !fn.name.includes("claim"))
    );
}

function serializeArg(
    arg: any,
    type: string | { type: "Array"; inner_type: string; length: number } | undefined,
    structs: Map<string, InternalStruct>
): Felt[] {
    if (arg && typeof arg === "object" && typeof arg.toFelts === "function") {
        return arg.toFelts();
    }

    if (typeof type === "string") {
        const struct = structs.get(type);
        if (struct && arg && typeof arg === "object" && !Array.isArray(arg)) {
            const felts: Felt[] = [];
            for (const field of struct.fields) {
                const fieldValue = arg[field.name];
                if (fieldValue === undefined) {
                    throw new Error(`Missing field "${field.name}" for struct "${type}"`);
                }
                felts.push(...serializeArg(fieldValue, field.type, structs));
            }
            return felts;
        }
        return [typeof arg === "bigint" ? arg : BigInt(arg)];
    }

    if (typeof type === "object" && type.type === "Array") {
        if (!Array.isArray(arg)) {
            throw new Error(`Expected array for parameter, got ${typeof arg}`);
        }
        const felts: Felt[] = [];
        for (const item of arg) {
            felts.push(...serializeArg(item, type.inner_type, structs));
        }
        return felts;
    }

    return [typeof arg === "bigint" ? arg : BigInt(arg)];
}

function serializeArgs(
    args: any[],
    fieldPaths: FieldPath[],
    structs: Map<string, InternalStruct>
): Felt[] {
    const felts: Felt[] = [];
    for (let i = 0; i < fieldPaths.length; i++) {
        const fieldPath = fieldPaths[i];
        const arg = args[i];
        felts.push(...serializeArg(arg, fieldPath.type, structs));
    }
    return felts;
}

export class Contract {
    private _contractId: Felt;
    private _abi: AbiFormat;
    private _provider: IContractProvider;
    private _signer?: ISigner;
    private _checkpointId: Felt;
    private _userId: Felt;
    private _decoder: RecursiveDecoder;
    private _merkleHelper: ReturnType<typeof createMerkleHelper>;
    private _stateProxies: Map<string, any> = new Map();
    private _functions: Map<string, InternalFunction> = new Map();
    private _structs: Map<string, InternalStruct> = new Map();
    private _internalContract: InternalContract;

    constructor(contractId: Felt, abi: AbiFormat, signerOrProvider: ISigner | IContractProvider, opts: ContractOptions) {
        this._contractId = contractId;
        this._abi = abi;
        this._checkpointId = opts.checkpointId;
        this._userId = opts.userId;

        if (this._checkpointId === undefined || this._checkpointId === null) {
            const available = Object.keys(opts).map((k) => `"${k}"`).join(", ");
            throw new Error(
                `Contract constructor received an invalid or missing "checkpointId". ` +
                `Received options keys: { ${available} }. ` +
                `Did you mean to pass { checkpointId: <value>, userId: <value> }?`
            );
        }
        if (this._userId === undefined || this._userId === null) {
            throw new Error(
                `Contract constructor received an invalid or missing "userId". ` +
                `Please ensure options includes { checkpointId: <value>, userId: <value> }.`
            );
        }

        if ("sendTransaction" in signerOrProvider && "getContractState" in signerOrProvider) {
            this._provider = signerOrProvider;
        } else if ("provider" in signerOrProvider) {
            this._signer = signerOrProvider;
            this._provider = signerOrProvider.provider;
        } else {
            throw new Error("Invalid signerOrProvider: must be either a Signer or Provider");
        }

        // Convert ABI
        const converter = new AbiConverter();
        const converted = converter.convert(abi);

        const contractStructs = abi.structs.filter((s) => s.is_contract);
        if (contractStructs.length === 0) {
            throw new Error("No contract found in ABI (no struct with is_contract: true)");
        }
        if (contractStructs.length > 1) {
            throw new Error("Multiple contracts in ABI not yet supported. Use codegen path.");
        }

        const contract = converted.contracts[0];
        this._internalContract = contract;

        for (const fn of contract.functions) {
            this._functions.set(fn.name, fn);
        }

        for (const struct of contract.structs) {
            this._structs.set(struct.name, struct);
        }

        this._decoder = new RecursiveDecoder();
        this._merkleHelper = createMerkleHelper(
            this._provider,
            this._checkpointId,
            this._contractId,
            this._userId
        );
        this._initializeStateVariables();

        // Return a Proxy that intercepts property access
        return new Proxy(this, {
            get(target, prop, receiver) {
                if (typeof prop !== "string") {
                    return Reflect.get(target, prop, receiver);
                }

                // Own properties / methods
                if (prop in target || target._functions.has(prop) || target._stateProxies.has(prop)) {
                    // If it's a method on the prototype, return it bound
                    const ownVal = Reflect.get(target, prop, receiver);
                    if (ownVal !== undefined) {
                        return ownVal;
                    }
                }

                // State variable
                if (target._stateProxies.has(prop)) {
                    return target._stateProxies.get(prop);
                }

                // Function dispatcher
                if (target._functions.has(prop)) {
                    return async (...args: any[]) => target._dispatchFunction(prop, args);
                }

                return Reflect.get(target, prop, receiver);
            },
        }) as Contract;
    }

    attach(signer: ISigner): Contract {
        return new Contract(
            this._contractId,
            this._abi,
            signer,
            { checkpointId: this._checkpointId, userId: this._userId }
        );
    }

    connect(signerOrProvider: ISigner | IContractProvider): Contract {
        return new Contract(
            this._contractId,
            this._abi,
            signerOrProvider,
            { checkpointId: this._checkpointId, userId: this._userId }
        );
    }

    updateCheckpoint(newCheckpointId: Felt): void {
        this._checkpointId = newCheckpointId;
        this._merkleHelper = createMerkleHelper(
            this._provider,
            this._checkpointId,
            this._contractId,
            this._userId
        );
        this._stateProxies.clear();
        this._initializeStateVariables();
    }

    withCheckpoint(newCheckpointId: Felt): Contract {
        return new Contract(
            this._contractId,
            this._abi,
            this._signer || this._provider,
            { checkpointId: newCheckpointId, userId: this._userId }
        );
    }

    async updateToLatest(): Promise<void> {
        if (this._provider.getLatestCheckpointId) {
            const latestCheckpointId = await this._provider.getLatestCheckpointId();
            this.updateCheckpoint(latestCheckpointId);
            return;
        }

        if ((this._provider as any).coordinatorEdgeRpcProvider?.getLatestBlockState) {
            const latestState = await (this._provider as any).coordinatorEdgeRpcProvider.getLatestBlockState();
            this.updateCheckpoint(latestState.checkpoint_id);
            return;
        }

        throw new Error(
            "Provider does not support getLatestCheckpointId(). Please use updateCheckpoint() with an explicit checkpoint ID."
        );
    }

    get checkpointId(): Felt {
        return this._checkpointId;
    }

    get signer(): ISigner | undefined {
        return this._signer;
    }

    get provider(): IContractProvider {
        return this._provider;
    }

    private _initializeStateVariables(): void {
        const variablePositions = this._internalContract.user_variable_positions as IFlatVariablePosition[];
        variablePositions.forEach((varPos) => {
            const proxy = createVariableProxy(this._merkleHelper, varPos, BigInt(0));
            this._stateProxies.set(varPos.name, proxy);
        });
    }

    private async _dispatchFunction(name: string, args: any[]): Promise<any> {
        const fn = this._functions.get(name);
        if (!fn) {
            throw new Error(`Unknown function: ${name}`);
        }

        const view = isViewFunction(fn);
        if (!view && !this._signer) {
            throw new Error("Signer required for state-changing functions. Use contract.attach(signer)");
        }

        const serializedArgs = serializeArgs(args, fn.field_flat_paths, this._structs);

        const result = await this._provider.sendTransaction(
            this._contractId,
            name,
            serializedArgs,
            this._signer?.publicKey
        );

        if (fn.return_size > 0) {
            return this._decoder.decodeReturnValue(result);
        }
    }
}
