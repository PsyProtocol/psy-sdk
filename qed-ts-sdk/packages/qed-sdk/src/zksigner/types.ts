import { ProofWithPublicInputs } from "../local-prover-rpc";
import { QHashOut } from "../types";

type TQedTransactionSignerAbility = "sign-hash" | "export-private-key-hex";

interface IQedTransactionSigner {
    getPublicKeyHex(): Promise<string>;
    getPrivateKeyHex?(): Promise<string>;
    signHash?(hash: QHashOut): Promise<ProofWithPublicInputs>;
    signAndSubmit(): Promise<string>;
    getAbilities(): TQedTransactionSignerAbility[];
}

export type { IQedTransactionSigner, TQedTransactionSignerAbility };
