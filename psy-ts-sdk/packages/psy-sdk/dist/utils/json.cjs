'use strict';

var stringify = require('../node_modules/.pnpm/lossless-json@4.3.0/node_modules/lossless-json/lib/esm/stringify.cjs');
var parse = require('../node_modules/.pnpm/lossless-json@4.3.0/node_modules/lossless-json/lib/esm/parse.cjs');
var utils = require('../node_modules/.pnpm/lossless-json@4.3.0/node_modules/lossless-json/lib/esm/utils.cjs');

const MAX_SAFE_INT = BigInt("9007199254740991");
// parse integer values into a bigint, and use a regular number otherwise
function customNumberParser(value) {
    return utils.isInteger(value) && BigInt(value) > MAX_SAFE_INT ? BigInt(value) : parseFloat(value);
}
function parseBigIntJson(jsonString) {
    return parse.parse(jsonString, null, customNumberParser);
}
function stringifyBigIntJSON(json, spaces = 0) {
    return (stringify.stringify(json, null, spaces, [
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

exports.PsyJSON = PsyJSON;
