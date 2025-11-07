export class OriginalFormatContractGenerator {
    constructor(private contract: any) {}

    generate(): string {
        const imports = this.generateImports();
        const className = this.contract.name;
        const stateVariables = this.generateStateVariables();
        const functions = this.generateFunctions();
        const helpers = this.generateHelpers();
        const variablePositionsConstant = this.generateVariablePositionsConstant();

        return `${imports}

export class ${className} {
  private _provider: IContractProvider;
  private _signer?: ISigner;
  private _contractId: GUint;
  private _userId: GUint;
  private _merkleHelper: IMerkleProxyHelper;
  private _decoder: RecursiveDecoder;
  private _stateProxies: Map<string, any> = new Map();

  constructor(userId: GUint, contractId: GUint, signerOrProvider: ISigner | IContractProvider) {
    this._userId = userId;
    this._contractId = contractId;
    
    // Handle both signer and provider inputs
    if ('sendTransaction' in signerOrProvider && 'getContractState' in signerOrProvider) {
      // It's a provider
      this._provider = signerOrProvider;
    } else if ('provider' in signerOrProvider) {
      // It's a signer
      this._signer = signerOrProvider;
      this._provider = signerOrProvider.provider;
    } else {
      throw new Error('Invalid signerOrProvider: must be either a Signer or Provider');
    }
    
    this._decoder = new RecursiveDecoder();
    
    // Initialize Merkle helper
    this._merkleHelper = this._createMerkleHelper();
    
    // Initialize state variables
    this._initializeStateVariables();
  }

  // Attach a signer to the contract
  attach(signer: ISigner): ${className} {
    const newContract = new ${className}(this._userId, this._contractId, signer);
    return newContract;
  }

  // Connect to a different provider
  connect(signerOrProvider: ISigner | IContractProvider): ${className} {
    return new ${className}(this._userId, this._contractId, signerOrProvider);
  }

  // Get the current signer
  get signer(): ISigner | undefined {
    return this._signer;
  }

  // Get the current provider
  get provider(): IContractProvider {
    return this._provider;
  }

  // State Variables
${stateVariables}

  // Contract Functions  
${functions}

  // Helper Methods
${helpers}

  // Variable Positions
${variablePositionsConstant}
}`;
    }

    private generateImports(): string {
        return `// Auto-generated from ABI - Do not edit manually
import { RecursiveDecoder } from './decoder';
import { GUint, BigNumberish, IContractProvider, ISigner } from './types';
import { keccak256, toBeHex, zeroPadValue } from 'ethers';

// Inline Merkle proxy types and implementation
interface IMerkleProxyHelper {
  add: (a: any, b: any) => any;
  mul: (a: any, b: any) => any;
  simplify: (x: any) => any;
  getHashGUint: (index: any) => any;
  setHashGUint: (index: any, value: any) => any;
  resolveGUint: (value: any) => any;
}

interface IFlatVariablePosition {
  name: string;
  offset: number | bigint;
  array_length: number | bigint;
  nth_size: number | bigint;
  children: IFlatVariablePosition[];
}

// Inline Merkle proxy implementation
const arrayVariableProxy = {
  get(target: any, prop: any, receiver: any) {
    if (prop === Symbol.iterator) {
      return function* () {
        for (let i = BigInt(0); i < target.position.array_length; i++) {
          yield target.helper.add(
            target.newOffsetIndex,
            target.helper.mul(target.position.nth_size, i)
          );
        }
      };
    }
    
    if (prop === 'length') {
      return target.position.array_length;
    }
    
    // For array element access, calculate the element's base offset
    const index = BigInt(prop);
    const elementOffset = target.helper.mul(target.position.nth_size, index);
    const elementBaseOffset = target.helper.add(target.newOffsetIndex, elementOffset);
    
    // Pass the array element (children[0]) to createVariableProxy
    // This should be the [] element
    return createVariableProxy(
      target.helper,
      target.position.children[0],
      elementBaseOffset
    );
  },
};

const structVariableProxy = {
  get(target: any, prop: any, receiver: any) {
    const child = target.position.children.find((x: any) => x.name === prop);
    if (!child) {
      throw new Error(\`Unknown property: \${prop}\`);
    }
    
    // For struct fields, add the field's offset to the current base
    // Make sure we're using the child's offset, not any nth_size
    const fieldOffset = target.helper.add(target.newOffsetIndex, BigInt(child.offset));
    
    return createVariableProxy(
      target.helper,
      child,
      fieldOffset
    );
  },
};

function isPrimitiveVariable(position: IFlatVariablePosition): boolean {
  return position.children.length === 0 && position.nth_size === BigInt(0);
}

function isArrayVariable(position: IFlatVariablePosition): boolean {
  return position.children.length === 1 && position.children[0].name === '[]';
}

function createVariableProxy(
  helper: IMerkleProxyHelper,
  position: IFlatVariablePosition,
  baseIndex: any
): any {
  // For primitive variables, the baseIndex already includes all necessary offsets
  if (isPrimitiveVariable(position)) {
    return helper.getHashGUint(baseIndex);
  }
  
  // Special handling for array elements marked with '[]'
  if (position.name === '[]') {
    // This is an array element template - don't add its offset as it's already included in baseIndex
    // The [] element itself is NEVER an array - it's either a primitive or a struct
    
    // Check if it has children (struct) or not (primitive)
    if (position.children.length === 0) {
      // Primitive array element
      return helper.getHashGUint(baseIndex);
    } else {
      // Struct array element - create struct proxy
      return new Proxy({ helper, position, newOffsetIndex: baseIndex }, structVariableProxy);
    }
  }
  
  // For other complex variables, add the offset
  const newOffsetIndex = position.offset === BigInt(0)
    ? baseIndex
    : helper.add(baseIndex, position.offset);
  
  if (isArrayVariable(position)) {
    return new Proxy({ helper, position, newOffsetIndex }, arrayVariableProxy);
  }
  
  return new Proxy({ helper, position, newOffsetIndex }, structVariableProxy);
}

function wrapMerkleProxyHelperBasicSimplifier(
  helper: IMerkleProxyHelper
): IMerkleProxyHelper {
  const isZero = (x: any): boolean => {
    return typeof x === 'number' ? x === 0
      : typeof x === 'bigint' ? x === BigInt(0)
      : typeof x === 'string' ? x === '0'
      : false;
  };

  const isOne = (x: any): boolean => {
    return typeof x === 'number' ? x === 1
      : typeof x === 'bigint' ? x === BigInt(1)
      : typeof x === 'string' ? x === '1'
      : false;
  };

  const isNumeric = (x: any): boolean => {
    return (
      typeof x === 'number' ||
      typeof x === 'bigint' ||
      (typeof x === 'string' && x.charCodeAt(0) >= 0x30 && x.charCodeAt(0) <= 0x39)
    );
  };

  const simplify = (x: any) => {
    if (typeof x === 'bigint') return x;
    else if (isNumeric(x)) return BigInt(x);
    else return helper.simplify(x);
  };

  const resolveGUint = (value: any) => {
    if (typeof value === 'bigint') return value;
    else if (isNumeric(value)) return BigInt(value);
    else if (typeof value === 'string') return helper.resolveGUint(value);
    else return value;
  };

  const add = (a: any, b: any) => {
    if (isZero(a)) return simplify(b);
    else if (isZero(b)) return simplify(a);
    else if (typeof a === 'bigint' && typeof b === 'bigint') return a + b;
    else if (isNumeric(a) && isNumeric(b)) return BigInt(a) + BigInt(b);
    else return helper.add(resolveGUint(a), resolveGUint(b));
  };

  const mul = (a: any, b: any) => {
    if (isZero(a) || isZero(b)) return BigInt(0);
    else if (isOne(a)) return simplify(b);
    else if (isOne(b)) return simplify(a);
    else if (typeof a === 'bigint' && typeof b === 'bigint') return a * b;
    else if (isNumeric(a) && isNumeric(b)) return BigInt(a) * BigInt(b);
    else return helper.mul(resolveGUint(a), resolveGUint(b));
  };

  return { add, mul, simplify, getHashGUint: helper.getHashGUint, setHashGUint: helper.setHashGUint, resolveGUint };
}`;
    }

    private generateStateVariables(): string {
        return this.contract.user_variable_positions
            .map((varPos: any) => {
                const getterName = varPos.name;

                if (!varPos.children || varPos.children.length === 0) {
                    // Simple variable
                    return `  get ${getterName}(): Promise<GUint> {
    const proxy = this._stateProxies.get('${getterName}');
    return proxy;
  }`;
                } else {
                    // Complex variable (array/struct)
                    return `  get ${getterName}() {
    return this._stateProxies.get('${getterName}');
  }`;
                }
            })
            .join("\n\n");
    }

    private generateFunctions(): string {
        if (!this.contract.functions || this.contract.functions.length === 0) {
            return "  // No functions defined";
        }

        return this.contract.functions
            .map((fn: any) => {
                const params = this.generateFunctionParams(fn);
                const hasReturn = fn.return_size > 0;
                const returnType = hasReturn ? "Promise<any>" : "Promise<void>";

                return `  async ${fn.name}(${params}): ${returnType} {
    // Check if we have a signer for state-changing functions
    const isViewFunction = ${this.isViewFunction(fn)};
    if (!isViewFunction && !this._signer) {
      throw new Error('Signer required for state-changing functions. Use contract.attach(signer)');
    }

    const result = await this._provider.sendTransaction(
      this._contractId,
      '${fn.name}',
      [${this.getFunctionArgNames(fn).join(", ")}],
      this._signer?.publicKey
    );${hasReturn ? "\n    return this._decoder.decodeReturnValue(result);" : ""}
  }`;
            })
            .join("\n\n");
    }

    private isViewFunction(fn: any): boolean {
        return (
            fn.name.startsWith("get_") ||
            fn.name.startsWith("view_") ||
            (fn.return_size > 0 &&
                !fn.name.includes("mint") &&
                !fn.name.includes("transfer") &&
                !fn.name.includes("claim"))
        );
    }

    private generateFunctionParams(fn: any): string {
        if (!fn.field_flat_paths || fn.field_flat_paths.length === 0) {
            return "";
        }

        return fn.field_flat_paths
            .map((field: any) => {
                const paramName = field.path[0] || "value";
                return `${paramName}: BigNumberish`;
            })
            .join(", ");
    }

    private getFunctionArgNames(fn: any): string[] {
        if (!fn.field_flat_paths || fn.field_flat_paths.length === 0) {
            return [];
        }

        return fn.field_flat_paths.map((field: any) => field.path[0] || "value");
    }

    private generateHelpers(): string {
        return `  private _createMerkleHelper(): IMerkleProxyHelper {
    const baseHelper: IMerkleProxyHelper = {
      add: (a: any, b: any) => BigInt(a) + BigInt(b),
      mul: (a: any, b: any) => BigInt(a) * BigInt(b),
      simplify: (x: any) => {
        if (typeof x === 'bigint') return x;
        if (typeof x === 'number') return BigInt(x);
        if (typeof x === 'string' && /^\\d+$/.test(x)) return BigInt(x);
        return x;
      },
      getHashGUint: async (index: any) => {
        // IMPORTANT: Pass the raw offset to the provider
        // The provider will convert offset -> slot
        const offset = this._calculateOffset(index);
        const data = await this._provider.getContractState(
          this._contractId,
          this._userId,
          [offset]  // Pass offset, not slot!
        );
        return data[0] || BigInt(0);
      },
      setHashGUint: async (index: any, value: any) => {
        throw new Error('Direct state writes not supported');
      },
      resolveGUint: (value: any) => BigInt(value)
    };
    
    return wrapMerkleProxyHelperBasicSimplifier(baseHelper);
  }

  private _calculateOffset(index: any): GUint {
    // This returns the raw offset without any conversion
    if (typeof index === 'bigint') return index;
    if (typeof index === 'number') return BigInt(index);
    if (typeof index === 'object' && index.base !== undefined && index.key !== undefined) {
      return this._keccak256(index.key, index.base);
    }
    return BigInt(index);
  }

  private _keccak256(key: GUint, base: GUint): GUint {
    const keyBytes = zeroPadValue(toBeHex(key), 32);
    const baseBytes = zeroPadValue(toBeHex(base), 32);
    const encoded = keyBytes + baseBytes.slice(2); // Remove '0x' from second value
    return BigInt(keccak256(encoded));
  }

  private _initializeStateVariables(): void {
    const variablePositions = this._getVariablePositions();
    
    variablePositions.forEach((varPos: any) => {
      // Don't pass the offset as baseIndex - let createVariableProxy handle it
      const proxy = createVariableProxy(this._merkleHelper, varPos, BigInt(0));
      this._stateProxies.set(varPos.name, proxy);
    });
  }`;
    }

    private generateVariablePositionsConstant(): string {
        const positions = this.contract.user_variable_positions;

        return `  private _getVariablePositions() {
    return ${JSON.stringify(
        positions,
        (key, value) => {
            // Convert numeric values to bigint literals
            if (key === "offset" || key === "array_length" || key === "nth_size") {
                return value.toString() + "n";
            }
            return value;
        },
        2
    ).replace(/"(\d+)n"/g, "$1n")}; // Convert back to bigint literals
  }`;
    }
}
