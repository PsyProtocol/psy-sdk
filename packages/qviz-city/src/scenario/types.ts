import { ICitySighashGroth16ProofResult } from "@qstudio/city-block";

interface ICSProofNode {
  id: string;
  dependencies: ICSProofNode[];
  is_ref?: boolean;
}
interface ISimpleCityBlock {
  stateTransitionRoot: ICSProofNode;
  sighashProofs: ICitySighashGroth16ProofResult[];
}

export type {
  ICSProofNode,
  ISimpleCityBlock,
}