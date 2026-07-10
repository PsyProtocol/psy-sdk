import { IHTTPClient, ISimpleHTTPRequest, ISimpleHTTPResponse } from "./types";

type FetchImplementation = (
    input: RequestInfo | URL,
    init?: RequestInit,
) => Promise<Response>;

class FetchHTTPClient implements IHTTPClient {
    private readonly fetchImplementation: FetchImplementation;

    constructor(fetchImplementation?: FetchImplementation) {
        if (fetchImplementation) {
            this.fetchImplementation = (input, init) => fetchImplementation(input, init);
        } else if (typeof globalThis.fetch === "function") {
            this.fetchImplementation = globalThis.fetch.bind(globalThis);
        } else {
            throw new Error("No fetch implementation provided");
        }
    }
    async sendRequest(request: ISimpleHTTPRequest): Promise<ISimpleHTTPResponse> {
        const result = await this.fetchImplementation(request.url, {
            method: request.method,
            headers: request.headers,
            body: request.body,
            credentials: request.credentials,
            signal: request.signal,
        });
        if (!result.ok) {
            if (request.responseType === "json") {
                try {
                    const body = await result.json();
                    return {
                        statusCode: result.status,
                        body,
                    };
                } catch (e) {
                    console.error("Error parsing JSON response", e);
                    return {
                        statusCode: result.status,
                        body: null,
                    };
                }
            }
        }
        switch (request.responseType) {
            case "json":
                return {
                    statusCode: result.status,
                    body: await result.json(),
                };
            case "text":
                return {
                    statusCode: result.status,
                    body: await result.text(),
                };
            default:
                return {
                    statusCode: result.status,
                    body: await result.arrayBuffer(),
                };
        }
    }
}

export { FetchHTTPClient };
