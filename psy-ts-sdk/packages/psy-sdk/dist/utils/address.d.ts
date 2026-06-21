declare const WITHDRAWAL_TYPE_P2PKH: bigint;
declare const WITHDRAWAL_TYPE_P2SH: bigint;
declare function getDecodedAddress(address: string): {
    publicKeyHash: Uint8Array;
    scriptTypeFlag: number;
};
export { getDecodedAddress, WITHDRAWAL_TYPE_P2PKH, WITHDRAWAL_TYPE_P2SH };
//# sourceMappingURL=address.d.ts.map