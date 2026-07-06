import { IHashOut } from "poseidon-goldilocks-lite";
import { IPsyClaimDepositRequest, IPsySigAction, IPsyTransferRequest, IPsyWithdrawalRequest } from "./types";
import { SCNumberLike } from "../core";
declare function getWithdrawalHashFromPublicKeyHash(value: SCNumberLike, publicKeyHash: Uint8Array, scriptTypeFlag: SCNumberLike): IHashOut;
declare function getWithdrawalHashFromAddress(value: SCNumberLike, address: string): IHashOut;
declare function getClaimDepositSigAction(request: IPsyClaimDepositRequest): IPsySigAction;
declare function getTransferSigAction(request: IPsyTransferRequest): IPsySigAction;
declare function getWithdrawalSigAction(request: IPsyWithdrawalRequest): IPsySigAction;
declare function computeSigActionHash(sigAction: IPsySigAction): IHashOut;
export { getWithdrawalHashFromPublicKeyHash, getWithdrawalHashFromAddress, getClaimDepositSigAction, getTransferSigAction, getWithdrawalSigAction, computeSigActionHash, };
//# sourceMappingURL=sighash.d.ts.map