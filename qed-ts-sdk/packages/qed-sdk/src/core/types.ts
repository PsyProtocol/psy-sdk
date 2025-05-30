type SCNumberLike = bigint | number | string;
type SCFelt = bigint | number;
type Felt = bigint | number;
type HexString = string;

type QedHash = string;
type Hash256 = string;
type Hash160 = string;
type CompressedPublicKeyHex = string;
type QProvingJobDataIDSerializedWrapped = string;
type PrivateKey = string;
type PublicKey = string;
type HashOut = string;
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

type QedMerkleProof = MerkleProofCore<QedHash>;
type QedDeltaMerkleProof = IDeltaMerkleProofCore<QedHash>;
type DeltaMerkleProofCore = IDeltaMerkleProofCore<QedHash>;

interface ISimpleKVPair<K, V> {
    key: K;
    value: V;
}

export type {
    QedHash,
    SCFelt,
    Felt,
    SCNumberLike,
    Hash256,
    Hash160,
    PrivateKey,
    PublicKey,
    HashOut,
    QHashOut,
    CompressedPublicKeyHex,
    QProvingJobDataIDSerializedWrapped,
    MerkleProofCore,
    IDeltaMerkleProofCore,
    DeltaMerkleProofCore,
    QedMerkleProof,
    QedDeltaMerkleProof,
    ISimpleKVPair,
    HexString,
    U8Bytes,
};
