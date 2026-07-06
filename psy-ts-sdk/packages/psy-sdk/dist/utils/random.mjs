function randomBytesFactory() {
    if (typeof globalThis.crypto === "undefined" || typeof globalThis.crypto.getRandomValues !== "function") {
        const randomBytes = globalThis.require("crypto").randomBytes;
        return (length) => randomBytes(length);
    }
    return (length) => globalThis.crypto.getRandomValues(new Uint8Array(length));
}
const cryptoRandomBytes = randomBytesFactory();

export { cryptoRandomBytes };
