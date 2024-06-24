import { readU64LEFromBytes } from "../byteView";
import { IBincodeDeserializeResult, IBincodeSerializeHelper } from "./types";

const MAX_ARRAY_LENGTH = 0xffffffff;


class BincodeArraySerializeHelper<T> implements IBincodeSerializeHelper<T[]> {
  baseHelper: IBincodeSerializeHelper<T>;
  constructor(baseHelper: IBincodeSerializeHelper<T>) {
    this.baseHelper = baseHelper;
  }
  deserializeObject(data: Uint8Array, offset = 0): IBincodeDeserializeResult<T[]> {
    if(data.length < 8){
      throw new Error("BincodeArraySerializeHelper: Invalid data length");
    }
    const length = Number(readU64LEFromBytes(data, offset).toString());
    if(length === 0) {
      return {
        result: [],
        nextOffset: offset + 8,
        readLength: 8,
      }
    }else if(length > MAX_ARRAY_LENGTH) {
      throw new Error(`BincodeArraySerializeHelper: Invalid array length ${length}, exceeds MAX_ARRAY_LENGTH (${length} > ${MAX_ARRAY_LENGTH})`);
    }else{
      let curOffset = offset + 8;
      const result: T[] = [];
      for(let i = 0; i < length; i++) {
        const tmpResult = this.baseHelper.deserializeObject(data, curOffset);
        result.push(tmpResult.result);
        curOffset += tmpResult.readLength;
      }
      return { result, nextOffset: curOffset, readLength: curOffset - offset };
    }
  }
  serializeObject(obj: T[], destination: Uint8Array, offset = 0): number {
    const length = obj.length;
    const view = new DataView(destination.buffer);
    let curOffset = offset;
    // we only support array lengths up to 2^32 - 1 anyways
    view.setUint32(curOffset, length, true);
    // bincode uses u64 for array lengths, but we only support up to 2^32 - 1
    curOffset += 8;
    for(let i = 0; i < length; i++) {
      curOffset += this.baseHelper.serializeObject(obj[i], destination, curOffset);
    }
    return curOffset - offset;
  }
  getSerializedSize(obj: T[]): number {
    let size = 8;
    for(let i = 0; i < obj.length; i++) {
      size += this.baseHelper.getSerializedSize(obj[i]);
    }
    return size;
  }
}

export {
  BincodeArraySerializeHelper,
}