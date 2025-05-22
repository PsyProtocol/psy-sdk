type Field = string | bigint; 
type Hash = string; 

export interface ExtensionField {
  elements: Field[]; 
}

export interface MerkleCap {
  digests: Hash[]; 
}

export interface FriQueryStep {
  evals: ExtensionField[]; 
  merkle_proof: {
    siblings: Hash[]; 
  };
}

export interface FriInitialTreeProof {
  evals_proofs: Array<[Field[], {siblings: Hash[]}]>;
}

export interface FriQueryRound {
  initial_trees_proof: FriInitialTreeProof;
  steps: FriQueryStep[];
}

export interface PolynomialCoeffs {
  coeffs: ExtensionField[]; 
}

export interface FriProof {
  commit_phase_merkle_caps: MerkleCap[]; 
  query_round_proofs: FriQueryRound[]; 
  final_poly: PolynomialCoeffs; 
  pow_witness: Field; 
}

export interface OpeningSet {
  constants: ExtensionField[]; 
  plonk_sigmas: ExtensionField[]; 
  wires: ExtensionField[]; 
  plonk_zs: ExtensionField[]; 
  plonk_zs_next: ExtensionField[]; 
  partial_products: ExtensionField[]; 
  quotient_polys: ExtensionField[]; 
  lookup_zs: ExtensionField[]; 
  lookup_zs_next: ExtensionField[]; 
}

export interface Proof {
  wires_cap: MerkleCap; 
  plonk_zs_partial_products_cap: MerkleCap; 
  quotient_polys_cap: MerkleCap; 
  openings: OpeningSet; 
  opening_proof: FriProof; 
}

export interface ProofWithPublicInputs {
  proof: Proof; 
  public_inputs: Field[];
}