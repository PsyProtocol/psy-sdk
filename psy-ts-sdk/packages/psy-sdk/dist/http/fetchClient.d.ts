import { IHTTPClient, ISimpleHTTPRequest, ISimpleHTTPResponse } from "./types";
declare class FetchHTTPClient implements IHTTPClient {
    fetchImplementation: any;
    constructor(fetchImplementation?: any);
    sendRequest(request: ISimpleHTTPRequest): Promise<ISimpleHTTPResponse>;
}
export { FetchHTTPClient };
//# sourceMappingURL=fetchClient.d.ts.map