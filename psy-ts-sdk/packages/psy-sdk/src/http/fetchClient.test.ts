import { afterEach, describe, expect, it, jest } from "@jest/globals";

import { FetchHTTPClient } from "./fetchClient";

const originalFetch = globalThis.fetch;

afterEach(() => {
    globalThis.fetch = originalFetch;
});

describe("FetchHTTPClient", () => {
    it("calls the native fetch implementation with the global object as receiver", async () => {
        const response = {
            ok: true,
            status: 200,
            json: jest.fn(async () => ({ checkpoint_id: "42" })),
        } as unknown as Response;
        const brandedFetch = jest.fn(function (this: typeof globalThis) {
            if (this !== globalThis) {
                throw new TypeError("Illegal invocation");
            }
            return Promise.resolve(response);
        }) as unknown as typeof fetch;
        globalThis.fetch = brandedFetch;

        const client = new FetchHTTPClient();
        const result = await client.sendRequest({
            url: "http://127.0.0.1:1337",
            method: "POST",
            responseType: "json",
        });

        expect(result).toEqual({
            statusCode: 200,
            body: { checkpoint_id: "42" },
        });
        expect(brandedFetch).toHaveBeenCalledTimes(1);
    });
});
