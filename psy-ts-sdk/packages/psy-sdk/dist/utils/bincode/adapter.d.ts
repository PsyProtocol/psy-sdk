import { IBincodeDeserializeResult, IBincodeSerializeHelper } from "./types";
interface ISimpleBincodeSerializable<T> {
    serializeBincode(obj: T): Uint8Array;
    deserializeBincode(data: Uint8Array): {
        result: T;
        readLength: number;
    };
    bincodeSerializedSize(obj: T): number;
}
declare class SimpleBincodeSerializer<T> implements IBincodeSerializeHelper<T> {
    simpleHelper: ISimpleBincodeSerializable<T>;
    constructor(simpleHelper: ISimpleBincodeSerializable<T>);
    serializeObject(obj: T, destination: Uint8Array, offset?: number): number;
    deserializeObject(data: Uint8Array, offset?: number): IBincodeDeserializeResult<T>;
    getSerializedSize(obj: T): number;
}
export type { ISimpleBincodeSerializable };
export { SimpleBincodeSerializer };
//# sourceMappingURL=adapter.d.ts.map