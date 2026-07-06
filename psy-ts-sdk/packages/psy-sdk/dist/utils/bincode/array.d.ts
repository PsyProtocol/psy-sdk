import { IBincodeDeserializeResult, IBincodeSerializeHelper } from "./types";
declare class BincodeArraySerializeHelper<T> implements IBincodeSerializeHelper<T[]> {
    baseHelper: IBincodeSerializeHelper<T>;
    constructor(baseHelper: IBincodeSerializeHelper<T>);
    deserializeObject(data: Uint8Array, offset?: number): IBincodeDeserializeResult<T[]>;
    serializeObject(obj: T[], destination: Uint8Array, offset?: number): number;
    getSerializedSize(obj: T[]): number;
}
export { BincodeArraySerializeHelper };
//# sourceMappingURL=array.d.ts.map