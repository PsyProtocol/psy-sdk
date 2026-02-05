export class DecoderGenerator {
    generate(): string {
        return `// Auto-generated decoder - Do not edit manually
import { Felt, Decodable } from './types';

export class RecursiveDecoder {
  private decoders: Map<string, (data: any) => any> = new Map();
  
  constructor() {
    this.registerDefaultDecoders();
  }
  
  private registerDefaultDecoders(): void {
    // Primitive type decoders
    this.decoders.set('uint256', (data) => BigInt(data));
    this.decoders.set('Felt', (data) => BigInt(data));
    this.decoders.set('bool', (data) => data !== BigInt(0));
    this.decoders.set('address', (data) => {
      const addr = data.toString(16).padStart(40, '0');
      return '0x' + addr;
    });
    
    // Array decoder
    this.decoders.set('array', (data) => {
      if (Array.isArray(data)) {
        return data.map(item => this.decode('element', [], item));
      }
      return [];
    });
  }
  
  /**
   * Decode a value based on its path and context
   */
  decode(varName: string, path: string[], rawValue: any): any {
    const type = this.inferType(varName, path);
    const decoder = this.decoders.get(type);
    
    if (decoder) {
      return decoder(rawValue);
    }
    
    // Default: return as BigInt
    return BigInt(rawValue);
  }
  
  /**
   * Decode function return values
   */
  decodeReturnValue(rawValue: any): any {
    if (Array.isArray(rawValue)) {
      return rawValue.map(v => this.decode('return', [], v));
    }
    return this.decode('return', [], rawValue);
  }
  
  /**
   * Infer type from variable name and path
   */
  private inferType(varName: string, path: string[]): string {
    // Check path components
    const fullPath = [varName, ...path].join('.');
    
    // Balance and amount fields are usually uint256
    if (fullPath.includes('balance') || 
        fullPath.includes('amount') ||
        fullPath.includes('value')) {
      return 'uint256';
    }
    
    // Address fields
    if (fullPath.includes('address') || 
        fullPath.includes('addr') ||
        fullPath.includes('owner')) {
      return 'address';
    }
    
    // Boolean fields
    if (fullPath.includes('is_') || 
        fullPath.includes('has_') ||
        fullPath.includes('enabled')) {
      return 'bool';
    }
    
    // Arrays
    if (path.some(p => !isNaN(Number(p)))) {
      return 'array';
    }
    
    // Default to uint256
    return 'uint256';
  }
  
  /**
   * Register a custom decoder for a specific type
   */
  registerDecoder(typeName: string, decoder: (data: any) => any): void {
    this.decoders.set(typeName, decoder);
  }
  
  /**
   * Register a struct decoder
   */
  registerStructDecoder<T>(
    structName: string, 
    fields: { name: string; type: string }[]
  ): void {
    this.decoders.set(structName, (data: any[]) => {
      const result: any = {};
      fields.forEach((field, index) => {
        result[field.name] = this.decode(field.name, [], data[index]);
      });
      return result as T;
    });
  }
}`;
    }
}
