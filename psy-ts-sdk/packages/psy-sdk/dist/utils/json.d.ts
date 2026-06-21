declare function parseBigIntJson(jsonString: string): any;
declare function stringifyBigIntJSON(json: any, spaces?: number): string;
declare const PsyJSON: Readonly<{
    parse: typeof parseBigIntJson;
    stringify: typeof stringifyBigIntJSON;
}>;
export { PsyJSON };
//# sourceMappingURL=json.d.ts.map