import { script as btcScript } from 'bitcoinjs-lib';
import {Buffer} from '../helpers/buffer';

function compileValue(value: string){
  if(value.startsWith("0x")){
    const rv = value.substring(2);
    if(rv.length % 2 !== 0){
      throw new Error(`Invalid hex: ${value}, must be even length`)
    }
    if(!/^[0-9a-fA-F]+$/.test(rv)){
      throw new Error(`Invalid hex: ${value}`)
    }
    return rv.toLowerCase();
  }else if(value.startsWith('"')){
    if(!value.endsWith('"')){
      throw new Error(`Invalid string: ${value}`)
    }
    return Buffer.from(new TextEncoder().encode(value.substring(1, value.length-1)));
  }else{
    const num = Number(value);
    if(isNaN(num) || Math.round(num) !== num){
      throw new Error(`Invalid number: ${value}`)
    }
    return (num>=0&&num<=16)?`OP_${num}`:btcScript.number.encode(num).toString("hex");
  }
}
function compileLine(line: string){
  const realLine = line.trim();
  if(realLine.charAt(0) === "<"){
    if(realLine.charAt(realLine.length-1)!==">"){
      throw new Error(`Invalid code: ${line}`)
    }
    return compileValue(realLine.substring(1, realLine.length-1).trim());
  }else{
    if(line.startsWith("OP_")){
      return line;
    }else{
      throw new Error(`Invalid script line: ${line}`)
    }
  }
}

function compileBitcoinScript(script: string){
  return script.split("\n").filter(x=>x.trim().length).map(compileLine).join(" ");
}

function compileBitcoinScriptBuf(script: string){
  return btcScript.fromASM(compileBitcoinScript(script));
}

export {
  compileBitcoinScript,
  compileBitcoinScriptBuf,
}