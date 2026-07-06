import { stringify } from '../node_modules/.pnpm/lossless-json@4.3.0/node_modules/lossless-json/lib/esm/stringify.mjs';
import { parse } from '../node_modules/.pnpm/lossless-json@4.3.0/node_modules/lossless-json/lib/esm/parse.mjs';
import { isInteger } from '../node_modules/.pnpm/lossless-json@4.3.0/node_modules/lossless-json/lib/esm/utils.mjs';

const MAX_SAFE_INT = BigInt("9007199254740991");
// parse integer values into a bigint, and use a regular number otherwise
function customNumberParser(value) {
    return isInteger(value) && BigInt(value) > MAX_SAFE_INT ? BigInt(value) : parseFloat(value);
}
function parseBigIntJson(jsonString) {
    return parse(jsonString, null, customNumberParser);
}
function stringifyBigIntJSON(json, spaces = 0) {
    return (stringify(json, null, spaces, [
        {
            test: (x) => typeof x === "bigint",
            stringify: (x) => x.toString(),
        },
    ]) || "");
}
const PsyJSON = Object.freeze({
    parse: parseBigIntJson,
    stringify: stringifyBigIntJSON,
});

export { PsyJSON };
