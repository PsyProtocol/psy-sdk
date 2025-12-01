export class OriginalFormatContractGenerator {
    constructor(private contract: any) {}

    generate(): string {
        const imports = this.generateImports();
        const className = this.contract.name;
        const stateVariables = this.generateStateVariables();
        const functions = this.generateFunctions();
        const helpers = this.generateHelpers();
        const variablePositionsConstant = this.generateVariablePositionsConstant();
        const structDefinitions = this.generateStructDefinitions();

        return `${imports}
${structDefinitions}

export class ${className} {
  private _provider: IContractProvider;
  private _signer?: ISigner;
  private _checkpointId: Felt;
  private _contractId: Felt;
  private _userId: Felt;
  private _merkleHelper: IMerkleProxyHelper;
  private _decoder: RecursiveDecoder;
  private _stateProxies: Map<string, any> = new Map();

  constructor(checkpointId: Felt, userId: Felt, contractId: Felt, signerOrProvider: ISigner | IContractProvider) {
    this._checkpointId = checkpointId;
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
    const newContract = new ${className}(this._checkpointId, this._userId, this._contractId, signer);
    return newContract;
  }

  // Connect to a different provider
  connect(signerOrProvider: ISigner | IContractProvider): ${className} {
    return new ${className}(this._checkpointId, this._userId, this._contractId, signerOrProvider);
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
import { Felt, IContractProvider, ISigner, PsyFixedArray } from './types';
import { keccak256, toBeHex, zeroPadValue } from 'ethers';

// Inline Merkle proxy types and implementation
interface IMerkleProxyHelper {
  add: (a: any, b: any) => any;
  mul: (a: any, b: any) => any;
  simplify: (x: any) => any;
  getHashFelt: (index: any) => any;
  setHashFelt: (index: any, value: any) => any;
  resolveFelt: (value: any) => any;
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
    return helper.getHashFelt(baseIndex);
  }
  
  // Special handling for array elements marked with '[]'
  if (position.name === '[]') {
    // This is an array element template - don't add its offset as it's already included in baseIndex
    // The [] element itself is NEVER an array - it's either a primitive or a struct
    
    // Check if it has children (struct) or not (primitive)
    if (position.children.length === 0) {
      // Primitive array element
      return helper.getHashFelt(baseIndex);
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

  const resolveFelt = (value: any) => {
    if (typeof value === 'bigint') return value;
    else if (isNumeric(value)) return BigInt(value);
    else if (typeof value === 'string') return helper.resolveFelt(value);
    else return value;
  };

  const add = (a: any, b: any) => {
    if (isZero(a)) return simplify(b);
    else if (isZero(b)) return simplify(a);
    else if (typeof a === 'bigint' && typeof b === 'bigint') return a + b;
    else if (isNumeric(a) && isNumeric(b)) return BigInt(a) + BigInt(b);
    else return helper.add(resolveFelt(a), resolveFelt(b));
  };

  const mul = (a: any, b: any) => {
    if (isZero(a) || isZero(b)) return BigInt(0);
    else if (isOne(a)) return simplify(b);
    else if (isOne(b)) return simplify(a);
    else if (typeof a === 'bigint' && typeof b === 'bigint') return a * b;
    else if (isNumeric(a) && isNumeric(b)) return BigInt(a) * BigInt(b);
    else return helper.mul(resolveFelt(a), resolveFelt(b));
  };

  return { add, mul, simplify, getHashFelt: helper.getHashFelt, setHashFelt: helper.setHashFelt, resolveFelt };
}`;
    }

    private generateStateVariables(): string {
        return this.contract.user_variable_positions
            .map((varPos: any) => {
                const getterName = varPos.name;

                if (!varPos.children || varPos.children.length === 0) {
                    // Simple variable
                    return `  get ${getterName}(): Promise<Felt> {
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
        
        const argNames = this.getFunctionArgNames(fn);
        const serializeCode = argNames.length > 0 ? `
          const serializedArgs: Felt[] = [];
          ${argNames.map(name => `
          if (typeof ${name} === 'object' && ${name} !== null && typeof ${name}.toFelts === 'function') {
            serializedArgs.push(...${name}.toFelts());
          } else {
            serializedArgs.push(${name} as Felt);
          }`).join('\n    ')}
          ` : "const serializedArgs: Felt[] = [];";

                return `  async ${fn.name}(${params}): ${returnType} {
    // Check if we have a signer for state-changing functions
    const isViewFunction = ${this.isViewFunction(fn)};
    if (!isViewFunction && !this._signer) {
      throw new Error('Signer required for state-changing functions. Use contract.attach(signer)');
    }
    ${serializeCode}
    const result = await this._provider.sendTransaction(
      this._contractId,
      '${fn.name}',
      serializedArgs,
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
                const tsType = this.getTypeScriptType(field.type);
                return `${paramName}: ${tsType}`;
            })
            .join(", ");
    }
  
  private getTypeScriptType(type: any): string {
    if (typeof type === 'string') {
      // Handle basic types and struct references
      switch (type) {
        case 'Felt':
        case 'felt':
        case 'u32':
          return 'Felt';
        case 'Bool':
        case 'bool':
          return 'boolean';
        default:
          // Assume it's a struct reference
          return type;
      }
    } else if (typeof type === 'object' && type.type === 'Array') {
      const innerType = this.getTypeScriptType(type.inner_type);
      // For fixed size arrays, use the PsyFixedArray type
      return `PsyFixedArray<${innerType}, ${type.length}>`;
    }
    
    // Default fallback
    return 'Felt';
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
      getHashFelt: async (index: any) => {
        // IMPORTANT: Pass the raw offset to the provider
        // The provider will convert offset -> slot
        const offset = this._calculateOffset(index);
        const data = await this._provider.getContractState(
          this._checkpointId,
          this._contractId,
          this._userId,
          [offset]  // Pass offset, not slot!
        );
        return data[0] || BigInt(0);
      },
      setHashFelt: async (index: any, value: any) => {
        throw new Error('Direct state writes not supported');
      },
      resolveFelt: (value: any) => BigInt(value)
    };
    
    return wrapMerkleProxyHelperBasicSimplifier(baseHelper);
  }

  private _calculateOffset(index: any): Felt {
    // This returns the raw offset without any conversion
    if (typeof index === 'bigint') return index;
    if (typeof index === 'number') return BigInt(index);
    if (typeof index === 'object' && index.base !== undefined && index.key !== undefined) {
      return this._keccak256(index.key, index.base);
    }
    return BigInt(index);
  }

  private _keccak256(key: Felt, base: Felt): Felt {
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

  private generateStructDefinitions(): string {
    let structDefinitions = '';
    const structs = this.contract.structs;
    console.log(structs[0]);

    for (const struct of structs) {
      if (!struct.is_contract) {
        if (structDefinitions) {
          structDefinitions += '\n\n';
        }
        structDefinitions += this.generateStructDefinition(struct);
      }
    }

    return structDefinitions;
  }

  private generateStructDefinition(struct: any): string {
  const structName = struct.name;

  const fieldDefinitions = struct.fields
    .map((field: any) => {
      const fieldName = field.name;
      const fieldType = this.getTypeScriptType(field.type);
      return `  ${fieldName}: ${fieldType};`;
    })
    .join('\n');

  const constructorParams = struct.fields
    .map((field: any) => `${field.name}: ${this.getTypeScriptType(field.type)}`)
    .join(', ');
  const constructorAssignments = struct.fields
    .map((field: any) => `this.${field.name} = ${field.name};`)
    .join('\n    ');
  const constructor = `
  constructor(${constructorParams}) {
    ${constructorAssignments}
  }`;

  const toFeltsBody = this.generateToFeltsBody(struct);
  const toFeltsMethod = `
  toFelts(): Felt[] {
    const felts: Felt[] = [];
    ${toFeltsBody}
    return felts;
  }`;

  return `export class ${structName} {
${fieldDefinitions}
${constructor}
${toFeltsMethod}
}`;
}

private generateToFeltsBody(struct: any): string {
  return struct.fields.map(field => {
    const fieldName = field.name;
    const fieldType = field.type;

    if (fieldType.type === 'Array') {
      const innerType = fieldType.inner_type;
      const isStruct = this.contract.structs.some(s => s.name === innerType);
      if (isStruct) {
        return `this.${fieldName}.forEach(item => { felts.push(...item.toFelts()); });`;
      } else {
        return `this.${fieldName}.forEach(item => { felts.push(item); });`;
      }
    }

    if (this.contract.structs.some(s => s.name === fieldType)) {
      return `felts.push(...this.${fieldName}.toFelts());`;
    }

    return `felts.push(this.${fieldName});`;
  }).join('\n    ');
}
}
