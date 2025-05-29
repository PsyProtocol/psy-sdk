import { FetchHTTPClient, IHTTPClient } from "../http";
import { CityJSON } from "../utils/json";

export class BaseProvider {
    httpClient: IHTTPClient;
    url: string;

    constructor(url: string, httpClient?: IHTTPClient) {
        this.httpClient = httpClient || new FetchHTTPClient();
        this.url = url;
    }

    async rpc<T>(method: string, params: any[], id = "1", jsonrpc = "2.0"): Promise<T> {
        const response = await this.httpClient.sendRequest({
            method: "POST",
            url: this.url,
            headers: {
                "Content-Type": "application/json",
            },
            body: CityJSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "text",
        });

        if (response.statusCode >= 400) {
            throw new Error("Error in RPC call: " + CityJSON.stringify(response.body));
        }
        const result = CityJSON.parse(response.body);
        if (result.error) {
            throw new Error("Error in RPC call: " + CityJSON.stringify(result.error));
        } else {
            return result.result as T;
        }
    }
}
