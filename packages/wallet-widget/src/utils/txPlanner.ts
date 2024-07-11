import { IUTXO, ICreateP2PKHParams, Transaction, createP2PKHTransaction, ITransactionOutput, decodeAddressFull, isP2PKHAddress, getP2PKHOutputScript, getP2SHOutputScript } from "doge-sdk";
//import { ICreateP2PKHParams, Transaction } from "doge-sdk/dist/types";
import { IQWidgetWallet } from "../types";
import { WalletWidgetRPC } from "./rpc/walletRPC";
const P2PKH_INPUT_SCRIPT_LENGTH = 107;
const P2PKH_OUTPUT_SCRIPT_LENGTH = 25;
const P2SH_OUTPUT_SCRIPT_LENGTH = 23;

interface ITransactionOutputAddress {
  value: number;
  address: string;
}
function varuintEncodingLength(n: number) {
  return n < 0xfd ? 1 : n <= 0xffff ? 3 : n <= 0xffffffff ? 5 : 9;
}

function getTxSize(inputScriptLength: number, numberOfInputs: number, outputScriptLengths: number[]): number {
  const base = 8;
  const headerSize = base + varuintEncodingLength(numberOfInputs) + varuintEncodingLength(outputScriptLengths.length);
  const inputsSize = (40 + inputScriptLength) * numberOfInputs;

  const outputsSize = outputScriptLengths.reduce((acc, output) => {
    return acc + 8 + output;
  }, 0);
  return headerSize + inputsSize + outputsSize;
}


function getStandardP2PKHTxSize(numberOfInputs: number, numberOfP2PKHOutputs: number, numberOfP2SHOutputs: number): number {
  const base = 8;
  const headerSize = base + varuintEncodingLength(numberOfInputs) + varuintEncodingLength(numberOfP2PKHOutputs+numberOfP2SHOutputs);
  const inputsSize = numberOfInputs * (40 + P2PKH_INPUT_SCRIPT_LENGTH);
  const outputsSize = numberOfP2PKHOutputs * (8 + P2PKH_OUTPUT_SCRIPT_LENGTH) + numberOfP2SHOutputs * (8 + P2SH_OUTPUT_SCRIPT_LENGTH);
  return headerSize + inputsSize + outputsSize;
}
function getCostOfAddingP2PKHInput(feeRate: number): number {
  return (40 + P2PKH_INPUT_SCRIPT_LENGTH)*feeRate;
}

function getValueOfUTXO(feeRate: number, value: number): number {
  return value - getCostOfAddingP2PKHInput(feeRate);
}

interface ITxPlanSettings {
  inputScriptLength: number;
  outputSelfScriptLength: number;
  outputs: {scriptLength: number, amount: number}[];
  feeRate: number;
  utxos: IUTXO[];
}
function coinSelect(settings: ITxPlanSettings): IUTXO[] {
  const { inputScriptLength, outputSelfScriptLength, outputs, feeRate, utxos } = settings;
  
  const targetAmount = outputs.reduce((sum, output) => sum + output.amount, 0);
  const dustThreshold = 546; // Typical dust threshold in satoshis
  const maxTries = 100000;
  const maxChangeOutputs = 3;
  
  function createSolution(): { utxos: IUTXO[], fee: number, change: number[] } | null {
    let selectedUTXOs: IUTXO[] = [];
    let selectedAmount = 0;
    let fee = 0;
    let changeOutputs: number[] = [];
    
    // Randomly select UTXOs
    const shuffledUTXOs = [...utxos].sort(() => Math.random() - 0.5);
    
    for (const utxo of shuffledUTXOs) {
      selectedUTXOs.push(utxo);
      selectedAmount += utxo.value;
      
      fee = getTxSize(
        inputScriptLength,
        selectedUTXOs.length,
        [...outputs.map(o => o.scriptLength), ...changeOutputs.map(() => outputSelfScriptLength)]
      ) * feeRate;
      
      if (selectedAmount >= targetAmount + fee) {
        let change = selectedAmount - (targetAmount + fee);
        
        // If change is less than dust, add it to the fee
        if (change > 0 && change < dustThreshold) {
          fee += change;
          change = 0;
        }
        
        // If we have change, create change outputs
        while (change > dustThreshold && changeOutputs.length < maxChangeOutputs) {
          const changeOutput = Math.min(change, Math.random() * change);
          changeOutputs.push(changeOutput);
          change -= changeOutput;
          
          // Recalculate fee with new change output
          fee = getTxSize(
            inputScriptLength,
            selectedUTXOs.length,
            [...outputs.map(o => o.scriptLength), ...changeOutputs.map(() => outputSelfScriptLength)]
          ) * feeRate;
        }
        
        // If we still have change, add it to the last change output
        if (change > 0 && changeOutputs.length > 0) {
          changeOutputs[changeOutputs.length - 1] += change;
        }
        
        return { utxos: selectedUTXOs, fee, change: changeOutputs };
      }
    }
    
    return null; // Not enough funds
  }
  
  let bestSolution: { utxos: IUTXO[], fee: number, change: number[] } | null = null;
  
  // Try to find the best solution
  for (let i = 0; i < maxTries; i++) {
    const solution = createSolution();
    if (solution) {
      if (!bestSolution || (
        solution.utxos.length <= bestSolution.utxos.length &&
        solution.fee <= bestSolution.fee &&
        solution.change.length <= bestSolution.change.length
      )) {
        bestSolution = solution;
      }
      
      // If we found a perfect solution, stop searching
      if (solution.change.length === 0 && solution.fee === Math.ceil(getTxSize(
        inputScriptLength,
        solution.utxos.length,
        outputs.map(o => o.scriptLength)
      ) * feeRate)) {
        break;
      }
    }
  }
  
  if (!bestSolution) {
    throw new Error("Insufficient funds");
  }
  
  return bestSolution.utxos as IUTXO[];
}
function getOutputScript(address: string): Uint8Array {
  const decoded = decodeAddressFull(address);
  if(isP2PKHAddress(address)){
    return getP2PKHOutputScript(decoded.hash);
  }else{
    return getP2SHOutputScript(decoded.hash);
  }
}
function coinSelectP2PKH(address: string, feeRate: number, utxos: IUTXO[], outputs: ITransactionOutputAddress[]): ICreateP2PKHParams & {fee: number} {
  const inputScriptLength = P2PKH_INPUT_SCRIPT_LENGTH;
  const outputSelfScriptLength = P2PKH_OUTPUT_SCRIPT_LENGTH;
  const settings: ITxPlanSettings = {
    inputScriptLength,
    outputSelfScriptLength,
    outputs: outputs.map(o => ({scriptLength: getOutputScript(o.address).length, amount: o.value})),
    feeRate,
    utxos,
  };
  const newOutputs = outputs.concat([]);
  const selectedUTXOs = coinSelect(settings);
  const selectedAmount = selectedUTXOs.reduce((sum, utxo) => sum + utxo.value, 0);
  const outputAmount = outputs.reduce((sum, x)=>sum+x.value, 0);
  const costOfChange = feeRate*(outputSelfScriptLength+8);
  if(selectedAmount < outputAmount){
    throw new Error("Insufficient funds");
  }
  if((selectedAmount - outputAmount) > costOfChange){
    newOutputs.push({value: Math.floor(selectedAmount - outputAmount - costOfChange), address});
  }
  console.log(newOutputs);
  const fee = getTxSize(
    inputScriptLength,
    selectedUTXOs.length,
    newOutputs.map(o => getOutputScript(o.address).length)
  ) * feeRate;

  return {
    address,
    inputs: selectedUTXOs,
    outputs: newOutputs,
    fee,
  };


}


export {
  coinSelectP2PKH,
}