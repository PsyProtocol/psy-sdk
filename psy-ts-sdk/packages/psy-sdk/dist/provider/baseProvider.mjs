import { FetchHTTPClient } from '../http/fetchClient.mjs';
import '../utils/felt.mjs';
import { PsyJSON } from '../utils/json.mjs';
import '../utils/random.mjs';

class BaseProvider {
    constructor(url, httpClient) {
        this.httpClient = httpClient || new FetchHTTPClient();
        this.url = url;
    }
    async rpc(method, params, id = "1", jsonrpc = "2.0") {
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
            throw new Error("Error in RPC call: " + PsyJSON.stringify(response.body));
        }
        const result = PsyJSON.parse(response.body);
        if (result.error) {
            throw new Error("Error in RPC call: " + PsyJSON.stringify(result.error));
        }
        else {
            return result.result;
        }
    }
}

export { BaseProvider };
