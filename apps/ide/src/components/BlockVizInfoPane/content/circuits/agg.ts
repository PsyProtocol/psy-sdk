import { getCircuitNameForJobId, IAggStateTransitionInput, IAggStateTransitionWithEventsInput, IQProvingJobDataID, ProvingJobCircuitType, PSCityBlock } from "@qstudio/city-block";
import { IBlockVizJobInfo, IRealBlockVizJobInfo } from "../types";
import { RichTextElemType, TRichTextContent } from "../../../RichTextRenderer/types";
import { CircuitDescriptions } from "../circuitInfo";
import {twoToOneHex} from "poseidon-goldilocks-lite";
import { TBVJobInfoGenerator } from "./types";

function wrapGenerateAggStateBVJobInfo(trees: string[]): TBVJobInfoGenerator {
  return (ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID)=>generateAggStateBVJobInfo(ctx, jobIdHex, jobId, trees);
}
async function generateAggStateBVJobInfo(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID, trees: string[]): Promise<IBlockVizJobInfo<keyof typeof ProvingJobCircuitType>> {
  const witness = await ctx.loadJobWitness(jobIdHex);

  const ast:  IAggStateTransitionInput = witness as IAggStateTransitionInput;

  const deps = ctx.getDirectDependencyJobs(jobIdHex);
  


  const summary: TRichTextContent = trees.length === 1 ? 
  [
    `Proves that the ${trees[0]} state tree transitions legally from its `,
    {type: RichTextElemType.Hash, hash: ast.left_input.state_transition_start, text: "old root"},
    ` to a valid `,
    {type: RichTextElemType.Hash, hash: ast.right_input.state_transition_end, text: "new root"},
  ] : [
    `Proves that the state trees ${trees.join(", ")} transition legally from their roots from an old root to a new root, showing that `,
    {type: RichTextElemType.Hash, hash: ast.left_input.state_transition_start, text: `Hash(${trees.map(x=>`${x}_start_root`).join(", ")})`},
    ` is valid hash of the start roots and `,
    {type: RichTextElemType.Hash, hash: ast.right_input.state_transition_end, text: `Hash(${trees.map(x=>`${x}_new_root`).join(", ")})`},
    ` is a valid hash of the new roots`
  ];

  const depsShortActions = (await Promise.all(deps.map(async dep => {
    const ji = await ctx.loadJobInfo(dep);
    return ji.shortActions;
  })));

  console.log("d,ds",deps,depsShortActions);

  const jobInfo: IBlockVizJobInfo<keyof typeof ProvingJobCircuitType> = {
    jobIdHex,
    jobId,
    circuitType: jobId.circuit_type,
    witness,
    dependencyJobs: deps,
    title: getCircuitNameForJobId(jobId),
    description: CircuitDescriptions[jobId.circuit_type],
    summary: summary,
    shortActions: depsShortActions.reduce((a,b)=>a.concat(b),[]),
    constraints: []
  };
  return jobInfo;

}

function wrapGenerateAggWithEventsStateBVJobInfo(trees: string[], eventHashes: string[]): TBVJobInfoGenerator {
  return (ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID)=>generateAggWithEventsStateBVJobInfo(ctx, jobIdHex, jobId, trees, eventHashes);
}

async function generateAggWithEventsStateBVJobInfo(ctx: PSCityBlock<IRealBlockVizJobInfo>, jobIdHex: string, jobId: IQProvingJobDataID, trees: string[], eventHashes: string[]): Promise<IBlockVizJobInfo<keyof typeof ProvingJobCircuitType>> {
  const witness = await ctx.loadJobWitness(jobIdHex);

  const ast:  IAggStateTransitionWithEventsInput = witness as IAggStateTransitionWithEventsInput;

  const deps = ctx.getDirectDependencyJobs(jobIdHex);
  const newEventHash = twoToOneHex(ast.left_input.event_hash, ast.right_input.event_hash);


  const summary: TRichTextContent = trees.length === 1 ? 
  [
    `Proves that the ${trees[0]} state tree transitions legally from its `,
    {type: RichTextElemType.Hash, hash: ast.left_input.state_transition_start, text: "old root"},
    ` to a valid `,
    {type: RichTextElemType.Hash, hash: ast.right_input.state_transition_end, text: "new root"},
    `.`
  ] : [
    `Proves that the state trees ${trees.join(", ")} transition legally from their roots from an old root to a new root, showing that `,
    {type: RichTextElemType.Hash, hash: ast.left_input.state_transition_start, text: `Hash(${trees.map(x=>`${x}_start_root`).join(", ")})`},
    ` is valid hash of the start roots and `,
    {type: RichTextElemType.Hash, hash: ast.right_input.state_transition_end, text: `Hash(${trees.map(x=>`${x}_new_root`).join(", ")})`},
    ` is a valid hash of the new roots.`
  ];

  const eventSummary: TRichTextContent = eventHashes.length === 1 ? [
    `The proof also proves that the combined ${eventHashes[0]} event hashes from the child proofs `,
    {type: RichTextElemType.Hash, hash: ast.left_input.event_hash, text: ast.left_input.event_hash},
    `, and `,
    {type: RichTextElemType.Hash, hash: ast.left_input.event_hash, text: ast.left_input.event_hash},
    ` are valid and combine to form a new event hash `,
    {type: RichTextElemType.Hash, hash: newEventHash, text: newEventHash},
    `.`
  ]:[

    `Proves that the combined ${eventHashes[0]} event hashes from the child proofs `,
    {type: RichTextElemType.Hash, hash: ast.left_input.event_hash, text: ast.left_input.event_hash},
    `, and `,
    {type: RichTextElemType.Hash, hash: ast.left_input.event_hash, text: ast.left_input.event_hash},
    ` are valid and combine to form a new event hash `,
    {type: RichTextElemType.Hash, hash: newEventHash, text: newEventHash},
    `.`
  ]

  const depsShortActions = (await Promise.all(deps.map(async dep => {
    const ji = await ctx.loadJobInfo(dep);
    return ji.shortActions;
  })));


  console.log("d,ds",deps,depsShortActions);
  const jobInfo: IBlockVizJobInfo<keyof typeof ProvingJobCircuitType> = {
    jobIdHex,
    jobId,
    circuitType: jobId.circuit_type,
    witness,
    dependencyJobs: deps,
    title: getCircuitNameForJobId(jobId),
    description: CircuitDescriptions[jobId.circuit_type],
    summary: summary.concat(eventSummary),
    shortActions: depsShortActions.reduce((a,b)=>a.concat(b),[]),
    constraints: []
  };
  return jobInfo;

}

export {
  generateAggStateBVJobInfo,
  generateAggWithEventsStateBVJobInfo,
  wrapGenerateAggStateBVJobInfo,
  wrapGenerateAggWithEventsStateBVJobInfo,
}