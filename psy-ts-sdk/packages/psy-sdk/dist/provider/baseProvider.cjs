'use strict';

var fetchClient = require('../http/fetchClient.cjs');
require('../utils/felt.cjs');
var json = require('../utils/json.cjs');
require('../utils/random.cjs');

class BaseProvider {
    constructor(url, httpClient) {
        this.httpClient = httpClient || new fetchClient.FetchHTTPClient();
        this.url = url;
    }
    async rpc(method, params, id = "1", jsonrpc = "2.0") {
        const response = await this.httpClient.sendRequest({
            method: "POST",
            url: this.url,
            headers: {
                "Content-Type": "application/json",
            },
            body: json.PsyJSON.stringify({
                jsonrpc,
                method,
                params,
                id,
            }),
            responseType: "text",
        });
        if (response.statusCode >= 400) {
            throw new Error("Error in RPC call: " + json.PsyJSON.stringify(response.body));
        }
        const result = json.PsyJSON.parse(response.body);
        if (result.error) {
            throw new Error("Error in RPC call: " + json.PsyJSON.stringify(result.error));
        }
        else {
            return result.result;
        }
    }
}

exports.BaseProvider = BaseProvider;
