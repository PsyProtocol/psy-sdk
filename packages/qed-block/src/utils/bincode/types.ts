interface IBincodeDeserializeResult<T> {
    result: T;
    nextOffset: number;
    readLength: number;
}
interface IBincodeSerializeHelper<T> {
    getSerializedSize(obj: T): number;
    deserializeObject(data: Uint8Array, offset?: number): IBincodeDeserializeResult<T>;
    serializeObject(obj: T, destination: Uint8Array, offset?: number): number;
}

export type { IBincodeDeserializeResult, IBincodeSerializeHelper };
