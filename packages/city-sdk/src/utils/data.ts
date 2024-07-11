/*

#[inline]
pub fn read_u32_be_at(array: &[u8], index: usize) -> u32 {
    ((array[index] as u32) << 24)
        + ((array[index + 1] as u32) << 16)
        + ((array[index + 2] as u32) << 8)
        + (array[index + 3] as u32)
}

#[inline]
pub fn read_u32_le_at(array: &[u8], index: usize) -> u32 {
    ((array[index + 3] as u32) << 24)
        + ((array[index + 2] as u32) << 16)
        + ((array[index + 1] as u32) << 8)
        + (array[index] as u32)
}

pub fn read_u48_from_bytes_le(bytes: &[u8], offset: usize) -> u64 {
    let mut result = 0u64;
    for i in 0..6 {
        result |= (bytes[offset + i] as u64) << (i * 8);
    }
    result
}

pub fn read_u56_from_bytes_le(bytes: &[u8], offset: usize) -> u64 {
    let mut result = 0u64;
    for i in 0..7 {
        result |= (bytes[offset + i] as u64) << (i * 8);
    }
    result
}


*/

function readU48FromBytesLE(bytes: Uint8Array, offset: number): number {
    let result = 0;
    for (let i = 0; i < 6; i++) {
        result |= bytes[offset + i] << (i * 8);
    }
    return result;
}
function readBigIntU48FromBytesLE(bytes: Uint8Array, offset: number): bigint {
    let result = BigInt(0);
    for (let i = 0; i < 6; i++) {
        result |= BigInt(bytes[offset + i]) << BigInt(i * 8);
    }
    return result;
}
function readBigIntU56FromBytesLE(bytes: Uint8Array, offset: number): bigint {
    let result = BigInt(0);
    for (let i = 0; i < 7; i++) {
        result |= BigInt(bytes[offset + i]) << BigInt(i * 8);
    }
    return result;
}

export {
  readBigIntU48FromBytesLE,
  readBigIntU56FromBytesLE,
  readU48FromBytesLE,
}