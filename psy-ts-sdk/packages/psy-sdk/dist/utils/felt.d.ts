import { IHashOut } from "poseidon-goldilocks-lite";
import { SCNumberLike } from "../core";
declare function psyFelt(x: SCNumberLike): bigint;
declare function cryptoRandomHashOut(): IHashOut;
declare function cryptoRandomHashOutHex(): string;
declare function hashOutHex(hashOut: IHashOut): string;
declare function hash256ToHashOut224(hashHex: string): IHashOut;
declare function publicKeyFeltsToBytes33(publicKey: SCNumberLike[]): Uint8Array;
declare function bytes33ToPublicKeyFelts(bytes: Uint8Array): bigint[];
declare function reverseHexBytes(hex: string): string;
declare function psyFeltSatsToDoge(x: SCNumberLike): string;
export { psyFelt, cryptoRandomHashOut, hashOutHex, cryptoRandomHashOutHex, hash256ToHashOut224, reverseHexBytes, psyFeltSatsToDoge, publicKeyFeltsToBytes33, bytes33ToPublicKeyFelts, };
//# sourceMappingURL=felt.d.ts.map