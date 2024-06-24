function randomBytesInsecure(length: number): Uint8Array {
  const result = new Uint8Array(length)
  for (let i = 0; i < length; i++) {
    result[i] = Math.floor(Math.random() * 256)
  }
  return result
}

export {
  randomBytesInsecure,
}