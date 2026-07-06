import { IBincodeSerializeHelper } from "./types";
declare function simpleBincodeSerialize<T>(helper: IBincodeSerializeHelper<T>, obj: T): Uint8Array;
declare function simpleBincodeDeserialize<T>(helper: IBincodeSerializeHelper<T>, data: Uint8Array): T;
export { simpleBincodeSerialize, simpleBincodeDeserialize };
//# sourceMappingURL=index.d.ts.map