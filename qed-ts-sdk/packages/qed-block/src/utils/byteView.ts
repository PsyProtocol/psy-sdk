function readU32LEFromBytes(bytes: Uint8Array | number[], offset = 0): number {
    return bytes[offset] + (bytes[offset + 1] << 8) + (bytes[offset + 2] << 16) + (bytes[offset + 3] << 24);
}
function writeU32LEToBytes(value: number, bytes: Uint8Array | number[], offset = 0): void {
    bytes[offset] = value & 0xff;
    bytes[offset + 1] = (value >> 8) & 0xff;
    bytes[offset + 2] = (value >> 16) & 0xff;
    bytes[offset + 3] = (value >> 24) & 0xff;
}
function readU16LEFromBytes(bytes: Uint8Array | number[], offset = 0): number {
    return bytes[offset] + (bytes[offset + 1] << 8);
}
function writeU16LEToBytes(value: number, bytes: Uint8Array | number[], offset = 0): void {
    bytes[offset] = value & 0xff;
    bytes[offset + 1] = (value >> 8) & 0xff;
}

function readU64LEFromBytes(bytes: Uint8Array | number[], offset = 0): bigint {
    return BigInt(readU32LEFromBytes(bytes, offset)) + (BigInt(readU32LEFromBytes(bytes, offset + 4)) << 32n);
}

function writeU64LEToBytes(value: bigint | string | number, bytes: Uint8Array | number[], offset = 0): void {
    const realValue = BigInt(value + "");
    writeU32LEToBytes(Number(realValue & 0xffffffffn), bytes, offset);
    writeU32LEToBytes(Number(realValue >> 32n), bytes, offset + 4);
}

export {
    readU32LEFromBytes,
    writeU32LEToBytes,
    readU16LEFromBytes,
    writeU16LEToBytes,
    readU64LEFromBytes,
    writeU64LEToBytes,
};
