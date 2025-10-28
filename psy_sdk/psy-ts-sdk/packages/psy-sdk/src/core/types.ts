type SCNumberLike = bigint | number | string;
type Felt = bigint | number;
type HexString = string;

type PsyHash = string;
type Hash256 = string;
type PrivateKey = string;
type PublicKey = string;
type QHashOut = string;
type U8Bytes = string | Uint8Array;

interface MerkleProofCore<T> {
    root: T;
    value: T;
    index: Felt;
    siblings: T[];
}

interface IDeltaMerkleProofCore<T> {
    old_root: T;
    old_value: T;
    new_root: T;
    new_value: T;
    index: Felt;
    siblings: T[];
}

type PsyMerkleProof = MerkleProofCore<PsyHash>;
type PsyDeltaMerkleProof = IDeltaMerkleProofCore<PsyHash>;
type DeltaMerkleProofCore = IDeltaMerkleProofCore<PsyHash>;

interface ISimpleKVPair<K, V> {
    key: K;
    value: V;
}

export type {
    PsyHash,
    Felt,
    SCNumberLike,
    Hash256,
    PrivateKey,
    PublicKey,
    QHashOut,
    MerkleProofCore,
    IDeltaMerkleProofCore,
    DeltaMerkleProofCore,
    PsyMerkleProof,
    PsyDeltaMerkleProof,
    ISimpleKVPair,
    HexString,
    U8Bytes,
};
