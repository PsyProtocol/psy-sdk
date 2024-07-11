import { parse, isInteger, stringify } from 'lossless-json'
const MAX_SAFE_INT = BigInt("9007199254740991");
// parse integer values into a bigint, and use a regular number otherwise
function customNumberParser(value: string) {
  return( isInteger(value) && (BigInt(value)>MAX_SAFE_INT) )? BigInt(value) : parseFloat(value)
}

function parseBigIntJson(jsonString: string): any {
  return parse(jsonString, null, customNumberParser)
}

function stringifyBigIntJSON(json: any, spaces: number = 0): string {
  return stringify(json, null, spaces, [{
    test: (x: any)=> typeof x === 'bigint',
    stringify: (x: any)=> x.toString(),
  }])||"";
}

const CityJSON = Object.freeze({
  parse: parseBigIntJson,
  stringify: stringifyBigIntJSON
});


export {
  CityJSON,
}