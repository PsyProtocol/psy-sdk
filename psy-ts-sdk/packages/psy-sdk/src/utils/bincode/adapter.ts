import { IBincodeDeserializeResult, IBincodeSerializeHelper } from "./types";

interface ISimpleBincodeSerializable<T> {
    serializeBincode(obj: T): Uint8Array;
    deserializeBincode(data: Uint8Array): { result: T; readLength: number };
    bincodeSerializedSize(obj: T): number;
}

class SimpleBincodeSerializer<T> implements IBincodeSerializeHelper<T> {
    simpleHelper: ISimpleBincodeSerializable<T>;
    constructor(simpleHelper: ISimpleBincodeSerializable<T>) {
        this.simpleHelper = simpleHelper;
    }
    serializeObject(obj: T, destination: Uint8Array, offset = 0): number {
        const data = this.simpleHelper.serializeBincode(obj);
        destination.set(data, offset);
        return data.length;
    }
    deserializeObject(data: Uint8Array, offset = 0): IBincodeDeserializeResult<T> {
        const { result, readLength } = this.simpleHelper.deserializeBincode(data.subarray(offset));
        return { result, nextOffset: offset + readLength, readLength };
    }
    getSerializedSize(obj: T): number {
        return this.simpleHelper.bincodeSerializedSize(obj);
    }
}

export type { ISimpleBincodeSerializable };

export { SimpleBincodeSerializer };
