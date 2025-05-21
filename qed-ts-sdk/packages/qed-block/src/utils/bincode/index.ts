import { IBincodeSerializeHelper } from "./types";

function simpleBincodeSerialize<T>(helper: IBincodeSerializeHelper<T>, obj: T): Uint8Array {
    const size = helper.getSerializedSize(obj);
    const result = new Uint8Array(size);
    helper.serializeObject(obj, result);
    return result;
}

function simpleBincodeDeserialize<T>(helper: IBincodeSerializeHelper<T>, data: Uint8Array): T {
    return helper.deserializeObject(data).result;
}

export { simpleBincodeSerialize, simpleBincodeDeserialize };
