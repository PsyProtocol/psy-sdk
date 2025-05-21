
function u8ArrayToHex(x: Uint8Array | number[]): string {
  const output: string[] = [];
  for (let i = 0, l = x.length; i < l; i++) {
    output[i] = x[i] < 0x10 ? ("0" + x[i].toString(16)) : x[i].toString(16);
  }
  return output.join("");
}

function hexToU8Array(hex: string): Uint8Array {
  let hexString = hex.charAt(1) === "x" ? hex.substring(2) : hex;
  if (hexString.length % 2 === 1) {
    throw new Error("hex strings must have an even number of characters");
  }
  const output = new Uint8Array(hexString.length / 2);
  for (let i = 0, l = hexString.length / 2; i < l; i++) {
    output[i] = parseInt(hexString.substring(i * 2, i * 2 + 2), 16);
  }

  return output;

}


function swapEndianU32(x: number){
  return ((((x&0xff)<<24)|((x&0xff00)<<8)|((x&0xff0000)>>>8)|(x>>>24))>>>0);
}


function swapEndianU16(x: number){
  return ((x>>>8) | ((x&0xff)<<8))>>>0;
}


function u32ArrayToHex(x: Uint32Array | ((number | bigint | string)[])): string {
  if(x instanceof Uint32Array){
    return u8ArrayToHex(new Uint8Array(x.buffer));
  }else{
    return u8ArrayToHex(new Uint8Array(new Uint32Array(x.map(x=>Number(x)).map(x=>((x<0?(x>>>0):x)&0xffffffff))).buffer));
  }
}
function hexToU32Array(hex: string, bigEndian = false): Uint32Array {
  const r = new Uint32Array(hexToU8Array(hex).buffer);
  if(bigEndian){
    return swapEndianU32Array(r);
  }else{
    return r;
  }
}


function u16ArrayToHex(x: Uint16Array | ((number | bigint | string)[])): string {
  if(x instanceof Uint16Array){
    return u8ArrayToHex(new Uint8Array(x.buffer));
  }else{
    return u8ArrayToHex(new Uint8Array(new Uint16Array(x.map(x=>Number(x)).map(x=>((x<0?(x>>>0):x)&0xffff))).buffer));
  }
}

function hexToU16Array(hex: string, bigEndian = false): Uint16Array {
  const r = new Uint16Array(hexToU8Array(hex).buffer);
  if(bigEndian){
    return swapEndianU16Array(r);
  }else{
    return r;
  }
}

function swapEndianU32Array(x: Uint32Array): Uint32Array;
function swapEndianU32Array(x: number[]): number[];
function swapEndianU32Array(x: Uint32Array | number[]){
  for(let i=0;i<x.length;i++){
    x[i] = swapEndianU32(x[i]);
  }
  return x;
}


function swapEndianU16Array(x: Uint16Array): Uint16Array;
function swapEndianU16Array(x: number[]): number[];
function swapEndianU16Array(x: Uint16Array | number[]){
  for(let i=0;i<x.length;i++){
    x[i] = swapEndianU16(x[i]);
  }
  return x;
}


function isZeroedArray(x: Uint8Array | number[]) {
  for (let i = 0; i < x.length; i++) {
    if (x[i] !== 0) {
      return false;
    }
  }
  return true;
}


function isDataValidASCII(array: Uint8Array | number[]): boolean {
  for (let i = 0; i < array.length; i++) {
    if (array[i]<0||array[i]>0x7f) {
      return false;
    }
  }
  return true;

}

function seq(count: number, startIndex = 0, reversed: number | boolean = false): number[] {
  const arr = [];
  if (reversed) {
    for (let i = 0; i < count; i++) {
      arr[i] = startIndex + count - 1 - i;
    }
  }else{
    for (let i = 0; i < count; i++) {
      arr[i] = startIndex + i;
    }
  }
  return arr;
}

function rseq(count: number, largestIndex?: number): number[] {
  if (typeof largestIndex !== 'number') {
    return seq(count, 0, true);
  }else{
    return seq(count, largestIndex-count+1, true);
  }
}

export {
  u8ArrayToHex,
  hexToU8Array,
  u32ArrayToHex,
  hexToU32Array,
  u16ArrayToHex,
  hexToU16Array,
  isZeroedArray,
  swapEndianU32Array,
  swapEndianU32,
  swapEndianU16Array,
  swapEndianU16,
  isDataValidASCII,
  seq,
  rseq,
}