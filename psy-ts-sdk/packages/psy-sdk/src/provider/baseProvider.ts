import { FetchHTTPClient, IHTTPClient } from "../http";
import { PsyJSON } from "../utils";

function formatRpcError(
    url: string,
    method: string,
    statusCode: number,
    body: unknown
): string {
    const rawBody = typeof body === "string" ? body.trim() : "";
    const bodyText = rawBody
        ? rawBody
        : body == null
            ? "<empty>"
            : PsyJSON.stringify(body);
    return `Error in RPC call ${method} ${url}: HTTP ${statusCode}, body=${bodyText || "<empty>"}`;
}

export class BaseProvider {
    httpClient: IHTTPClient;
    url: string;

    constructor(url: string, httpClient?: IHTTPClient) {
        this.httpClient = httpClient || new FetchHTTPClient();
        this.url = url;
    }

    async rpc<T>(method: string, params: any, id = "1", jsonrpc = "2.0"): Promise<T> {
        const response = await this.httpClient.sendRequest({
            method: "POST",
            url: this.url,
            headers: {
                "Content-Type": "application/json",
            },
            body: PsyJSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "text",
        });

        if (response.statusCode >= 400) {
            throw new Error(formatRpcError(this.url, method, response.statusCode, response.body));
        }
        const result = PsyJSON.parse(response.body);
        if (result.error) {
            throw new Error("Error in RPC call: " + PsyJSON.stringify(result.error));
        } else {
            return result.result as T;
        }
    }
}
