import { Psbt, Transaction, script } from "bitcoinjs-lib";
import { DogeLinkRPC } from "../rpc/link";
import { ISpendOutput, IFundingUTXO, IFinalizedFundingUTXO, IPsbtInputExtended, IPsbtOutputExtended, IBaseUTXO, IUTXOWithRawTransaction } from "../types/network";
import { getLinkSignerFromWalletProvider, getLinkSignerFromWalletSigner } from "../helpers/signer";
import { PsbtInput } from "bip174/src/lib/interfaces";
import { ISimpleDogeWalletProvider } from "../types/wallet";

function normalizeFundingUTXOs(fundingUTXOs: IFundingUTXO[]): IFinalizedFundingUTXO[] {
  
  return fundingUTXOs.map(x => {
    return {
      ...x,
      rawTransaction: typeof x.rawTransaction === 'string'?Buffer.from(x.rawTransaction, "hex"):x.rawTransaction,
    }
  });
}
function normalizeSpendOutputs(outputs: ISpendOutput[]): IPsbtOutputExtended[] {
  return outputs.map(x => {
    const script = typeof (x as any).script === 'string' ? Buffer.from((x as any).script, "hex") : (x as any).script;
    return {
      ...x,
      script,
    }
  });
}
function generateInputsForScriptSpend(fundingUTXOs: IFinalizedFundingUTXO[]): IPsbtInputExtended[]{

  
  const fundedUTXOs = normalizeFundingUTXOs(fundingUTXOs);
  const results: IPsbtInputExtended[] = [];
  for(let i=0;i<fundedUTXOs.length;i++){
    const fundedUTXO = fundedUTXOs[i];
    const txDataBuffer = fundedUTXO.rawTransaction;
    const input = {
      hash: fundedUTXO.txid,
      index: fundedUTXO.vout,
      nonWitnessUtxo: txDataBuffer,
      redeemScript: fundedUTXO.redeemScript,
    };
    results.push(input);
  }
  return results;
}
function standardWitnessGenerator(redeemScript: Buffer, scriptSolution: (Buffer|Uint8Array|string)[], inputIndex: number, input: PsbtInput, psbt: Psbt, signatures: Buffer[]): Buffer {
  const normalizedSolution = scriptSolution.map(x => typeof x === 'string' ? Buffer.from(x, "hex") : Buffer.from(x));

  const outs = signatures.map(x=>x.toString("hex"));
  if(normalizedSolution.length){
    outs.push(script.toASM(normalizedSolution));
  }
  if(redeemScript.length){
    outs.push(redeemScript.toString("hex"));
  }
  return script.fromASM(outs.join(" "));
}
async function finalizePSBT(
  rpc: DogeLinkRPC,
  inputs: IFundingUTXO[],
  outputs: ISpendOutput[],
  redeemScript: Buffer | Uint8Array | string,
  getScriptSolution: (inputIndex: number, input: PsbtInput, psbt: Psbt) => (Buffer|Uint8Array|string)[],
  sighashTypes: number[],
  finalizer: (redeemScript: Buffer, scriptSolution: (Buffer|Uint8Array|string)[], inputIndex: number, input: PsbtInput, psbt: Psbt, signatures: Buffer[]) => Buffer = standardWitnessGenerator,
): Promise<Psbt> {
  const normalizedRedeemScript = typeof redeemScript === 'string' ? Buffer.from(redeemScript, "hex") : Buffer.from(redeemScript);
  
  const network = rpc.getNetwork();
  const psbt = new Psbt({ network });
  const normalizedInputs = normalizeFundingUTXOs(inputs);
  const finalInputs = generateInputsForScriptSpend(normalizedInputs);
  const finalOutputs = normalizeSpendOutputs(outputs);



  psbt.addInputs(finalInputs);
  psbt.addOutputs(finalOutputs);

  const signers = normalizedInputs.map((x)=>(x.signers??[]).map(s=>getLinkSignerFromWalletSigner(s, psbt, sighashTypes)));
  for(let i=0;i<normalizedInputs.length;i++){
    for(let s=0;s<signers[i].length;s++){
      await psbt.signInputAsync(i, signers[i][s]);
    }
  }
  const customFinalizer = (inputIndex: number, input: PsbtInput) => {
    const signatures = (input.partialSig && input.partialSig.length) ? input.partialSig.map(x=>x.signature) : [];
    return {
      finalScriptSig: finalizer(normalizedRedeemScript, getScriptSolution(inputIndex, input, psbt), inputIndex, input, psbt, signatures),
      finalScriptWitness: undefined,
    }
  };

  for(let i=0;i<inputs.length;i++){
    psbt.finalizeInput(i, customFinalizer);
  }
  return psbt;
}

async function finalizePSBTStandard(
  rpc: DogeLinkRPC,
  inputs: IFundingUTXO[],
  outputs: ISpendOutput[],
  redeemScript: Buffer | Uint8Array | string,
  solutions: (Buffer|Uint8Array|string)[][],
  sighashTypes: number[],
): Promise<Psbt> {
  return finalizePSBT(rpc, inputs, outputs, redeemScript, (inputIndex, input, psbt) => solutions[inputIndex], sighashTypes);
}
async function finalizePSBTStandardSingleSolution(
  rpc: DogeLinkRPC,
  inputs: IFundingUTXO[],
  outputs: ISpendOutput[],
  redeemScript: Buffer | Uint8Array | string,
  solution: (Buffer|Uint8Array|string)[],
  sighashTypes: number[],
): Promise<Psbt> {
  return finalizePSBT(rpc, inputs, outputs, redeemScript, (inputIndex, input, psbt) => solution, sighashTypes);
}

async function spendMulti(
  rpc: DogeLinkRPC,
  inputs: IFundingUTXO[],
  outputs: ISpendOutput[],
  redeemScript: Buffer | Uint8Array | string,
  scriptSolutions: (Buffer|Uint8Array|string)[][],
  sighashTypes: number[] = [Transaction.SIGHASH_ALL],
) {
  const psbt = await finalizePSBTStandard(rpc, inputs, outputs, redeemScript, scriptSolutions, sighashTypes);
  return rpc.sendRawTransaction(psbt.extractTransaction().toHex());
}


async function spendSingle(
  rpc: DogeLinkRPC,
  inputs: IFundingUTXO[],
  outputs: ISpendOutput[],
  redeemScript: Buffer | Uint8Array | string,
  scriptSolutions: (Buffer|Uint8Array|string)[],
  sighashTypes: number[] = [Transaction.SIGHASH_ALL],
) {
  const psbt = await finalizePSBTStandardSingleSolution(rpc, inputs, outputs, redeemScript, scriptSolutions, sighashTypes);
  return rpc.sendRawTransaction(psbt.extractTransaction().toHex());
}

async function spendP2PKH(
  compressedPublicKeyHex: string,
  walletProvider: ISimpleDogeWalletProvider,
  rpc: DogeLinkRPC,
  inputs: IBaseUTXO[],
  outputs: ISpendOutput[],
  sighashTypes: number[] = [Transaction.SIGHASH_ALL],
) {
  const resolvedTransactions = await Promise.all(inputs.map(x=>rpc.getRawTransaction(x.txid).then(txHex=>Buffer.from(txHex, 'hex'))));
  const finalOutputs = normalizeSpendOutputs(outputs);

  const signer = getLinkSignerFromWalletProvider(compressedPublicKeyHex, walletProvider);
  const psbt = new Psbt({ network: rpc.getNetwork() });
  const normalizedInputs = inputs.map((x, i) => {
    return {
      hash: x.txid,
      index: x.vout,
      nonWitnessUtxo: resolvedTransactions[i],
    }
  });
  psbt.addInputs(normalizedInputs);
  psbt.addOutputs(finalOutputs);
  for(let i=0;i<inputs.length;i++){
    await psbt.signInputAsync(i, signer, sighashTypes);
  }
  psbt.finalizeAllInputs();
  const hex = psbt.extractTransaction(true).toHex();
  console.log("txHex",hex);
  return rpc.sendRawTransaction(hex);
}
export {
  spendMulti,
  spendSingle,
  spendP2PKH,
}