import { IHTTPClient } from "../http";
export declare class BaseProvider {
    httpClient: IHTTPClient;
    url: string;
    constructor(url: string, httpClient?: IHTTPClient);
    rpc<T>(method: string, params: any, id?: string, jsonrpc?: string): Promise<T>;
}
//# sourceMappingURL=baseProvider.d.ts.map