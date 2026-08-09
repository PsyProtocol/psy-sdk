import { describe, expect, it } from "@jest/globals";

import { PsyJSON } from "../utils";
import type { SimulatedTxJson, ViewCallData, ViewCallResult } from "./types";

describe("simulateContractCall / callView response contracts", () => {
    it("parses a generated simulation response with transaction metadata", () => {
        const raw = `{
            "generated": {
                "user_id": "1",
                "pk_hash": "0xabc",
                "sig_hash": "0xdef",
                "tx_hash": "0x123",
                "call_data": {
                    "contract_calls": [
                        {
                            "contract_id": 6,
                            "method_name": "mint",
                            "inputs": [100]
                        }
                    ],
                    "software_defined_call": { "inputs": [] }
                },
                "tx_count": 1,
                "trace": { "encoding": "json", "payload": "{}" }
            },
            "metadata": {
                "tx_hash": "0x123",
                "end_cap_data": {
                    "checkpoint_id": 9,
                    "user_id": 1,
                    "global_user_tree_height": 32,
                    "start_user_leaf_hash": "0xaaa",
                    "end_user_leaf_hash": "0xbbb",
                    "checkpoint_tree_root_hash": "0xccc",
                    "stats": {
                        "fees_collected": 0,
                        "user_ops_processed": 1,
                        "total_transactions": 1,
                        "slots_modified": 0
                    }
                },
                "contract_call_data": {
                    "contract_calls": [
                        {
                            "contract_id": 6,
                            "method_name": "mint",
                            "inputs": [100],
                            "outputs": []
                        }
                    ],
                    "software_defined_call": { "inputs": [] }
                },
                "storage_data": {
                    "reads": [],
                    "writes": []
                }
            }
        }`;

        const parsed = PsyJSON.parse(raw) as SimulatedTxJson;

        expect(parsed.generated).toBeDefined();
        if (!parsed.generated || !parsed.metadata.end_cap_data) {
            throw new Error("expected generated simulation metadata");
        }
        expect(parsed.generated.tx_hash).toBe("0x123");
        expect(parsed.generated.trace.encoding).toBe("json");
        expect(parsed.metadata.tx_hash).toBe("0x123");
        expect(parsed.metadata.end_cap_data.checkpoint_id).toBe(9);
        expect(parsed.metadata.contract_call_data.contract_calls[0].method_name).toBe("mint");
        expect(parsed.metadata.storage_data.writes).toEqual([]);
        expect("generated" in parsed).toBe(true);
        expect(Object.prototype.hasOwnProperty.call(parsed.metadata, "tx_hash")).toBe(true);
        expect(Object.prototype.hasOwnProperty.call(parsed.metadata, "end_cap_data")).toBe(true);
    });

    it("parses a fee-free simulation response with omitted transaction fields", () => {
        const raw = `{
            "metadata": {
                "contract_call_data": {
                    "contract_calls": [],
                    "software_defined_call": {}
                },
                "storage_data": {
                    "reads": [],
                    "writes": []
                }
            }
        }`;

        const parsed = PsyJSON.parse(raw) as SimulatedTxJson;

        expect(parsed.generated).toBeUndefined();
        expect(parsed.metadata.tx_hash).toBeUndefined();
        expect(parsed.metadata.end_cap_data).toBeUndefined();
        expect(parsed.metadata.storage_data.writes).toEqual([]);
    });

    it("parses a fixed view response without transaction preview fields", () => {
        const raw = `{
            "checkpoint_id": 42,
            "contract_calls": [
                {
                    "contract_id": 6,
                    "method_name": "get_counter",
                    "inputs": [],
                    "outputs": [7]
                }
            ],
            "storage_reads": [
                {
                    "user_id": 1,
                    "contract_id": 6,
                    "slot_index": 0,
                    "value": "0xddd"
                }
            ]
        }`;

        const parsed = PsyJSON.parse(raw) as ViewCallResult;

        expect(parsed.checkpoint_id).toBe(42);
        expect(parsed.contract_calls).toHaveLength(1);
        expect(parsed.contract_calls[0].method_name).toBe("get_counter");
        expect(parsed.contract_calls[0].outputs[0]).toBe(7);
        expect(parsed.storage_reads).toHaveLength(1);
        expect(parsed.storage_reads[0].value).toBe("0xddd");

        const keys = Object.keys(parsed).sort();
        expect(keys).toEqual(["checkpoint_id", "contract_calls", "storage_reads"]);
        expect(Object.prototype.hasOwnProperty.call(parsed, "generated")).toBe(false);
        expect(Object.prototype.hasOwnProperty.call(parsed, "metadata")).toBe(false);
        expect(Object.prototype.hasOwnProperty.call(parsed, "tx_hash")).toBe(false);
        expect(Object.prototype.hasOwnProperty.call(parsed, "end_cap_data")).toBe(false);
    });

    it("keeps view request payload free of software_defined_call", () => {
        const request: ViewCallData = {
            contract_calls: [
                {
                    contract_id: 6n,
                    method_name: "get_counter",
                    inputs: [],
                },
            ],
        };

        const encoded = PsyJSON.parse(PsyJSON.stringify(request)) as Record<string, unknown>;
        expect(Object.keys(encoded).sort()).toEqual(["contract_calls"]);
        expect(Object.prototype.hasOwnProperty.call(encoded, "software_defined_call")).toBe(false);
    });
});
