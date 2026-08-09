import { parse, isInteger, stringify } from "lossless-json";
const MAX_SAFE_INT = BigInt("9007199254740991");
const MIN_SAFE_INT = BigInt("-9007199254740991");
// parse integer values outside the safe range into a bigint; keep safe integers as number
function customNumberParser(value: string) {
    if (!isInteger(value)) return parseFloat(value);
    const asBigInt = BigInt(value);
    return asBigInt > MAX_SAFE_INT || asBigInt < MIN_SAFE_INT ? asBigInt : parseFloat(value);
}

function parseBigIntJson(jsonString: string): any {
    return parse(jsonString, null, customNumberParser);
}

function stringifyBigIntJSON(json: any, spaces: number = 0): string {
    return (
        stringify(json, null, spaces, [
            {
                test: (x: any) => typeof x === "bigint",
                stringify: (x: any) => x.toString(),
            },
        ]) || ""
    );
}

const PsyJSON = Object.freeze({
    parse: parseBigIntJson,
    stringify: stringifyBigIntJSON,
});

export { PsyJSON };
