/**
 * Structured-clone-safe codec for PsyWasmWebProverProvider call args/results.
 *
 * The unified app runs the WASM prover on a dedicated Web Worker (prover-
 * worker.ts) so multi-second proofs never block the UI thread. Worker messages
 * go through postMessage's structured clone, which CANNOT carry BigInt inside
 * typed arrays, wasm-bindgen pointer objects, or class instances. This codec
 * (ported verbatim from psy-wallet's offscreen/wasm-worker.ts) tags those
 * shapes into plain JSON-cloneable values on the way out and rebuilds them on
 * the way in.
 *
 * SHARED on purpose: both the worker and the main-thread relay import this ONE
 * module so a serialize/deserialize change can never drift between the two
 * sides (the silent-corruption risk if each kept its own copy).
 */

export type SerializedWasmValue =
  | null
  | boolean
  | number
  | string
  | SerializedWasmValue[]
  | { __psyType: "undefined" }
  | { __psyType: "bigint"; value: string }
  | { __psyType: "biguint64array"; values: string[] }
  | { __psyType: "uint8array"; base64?: string; values?: number[] }
  | { [key: string]: SerializedWasmValue };

export type WasmWorkerRequest = {
  id: number;
  /**
   * "provider" (default) dispatches against a PsyWasmWebProverProvider INSTANCE
   * method (execContractCall, claimBatch, registerUser, …). "static" dispatches
   * against the static PsyWasmWebProverProvider.wasmServer — the inner
   * WasmRpcServer — for methods the provider class doesn't re-expose as instance
   * methods, notably prove_private_note_inclusion_json (the private-note
   * inclusion proof). Both run on the SAME worker, so the static server shares
   * the signer registry registerUser installed.
   */
  mode?: "provider" | "static";
  method: string;
  configJson: string;
  args?: SerializedWasmValue[];
};

export type WasmWorkerReply = {
  id: number;
  ok: boolean;
  result?: SerializedWasmValue;
  error?: string;
};

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  for (let i = 0; i < bytes.length; i += 1) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function base64ToBytes(base64: string): Uint8Array {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

export function deserializeWasmValue(value: SerializedWasmValue): unknown {
  if (value === null) return null;
  if (
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  ) {
    return value;
  }
  if (Array.isArray(value)) return value.map(deserializeWasmValue);
  if ("__psyType" in value) {
    const tagged = value as {
      __psyType: "undefined" | "bigint" | "biguint64array" | "uint8array";
      value?: string;
      base64?: string;
      values?: string[] | number[];
    };
    if (tagged.__psyType === "undefined") return undefined;
    if (tagged.__psyType === "bigint") return BigInt(tagged.value ?? "0");
    if (tagged.__psyType === "biguint64array") {
      return BigUint64Array.from(tagged.values as string[], (item) =>
        BigInt(item),
      );
    }
    if (tagged.__psyType === "uint8array") {
      if (tagged.base64) return base64ToBytes(tagged.base64);
      return Uint8Array.from(tagged.values as number[]);
    }
  }
  return Object.fromEntries(
    Object.entries(value).map(([key, item]) => [
      key,
      deserializeWasmValue(item as SerializedWasmValue),
    ]),
  );
}

export function serializeWasmValue(value: unknown): SerializedWasmValue {
  if (value === undefined) return { __psyType: "undefined" };
  if (value === null) return null;
  if (typeof value === "bigint") {
    return { __psyType: "bigint", value: value.toString() };
  }
  if (
    typeof value === "string" ||
    typeof value === "number" ||
    typeof value === "boolean"
  ) {
    return value;
  }
  if (value instanceof BigUint64Array) {
    return {
      __psyType: "biguint64array",
      values: Array.from(value, (item) => item.toString()),
    };
  }
  if (value instanceof Uint8Array) {
    return { __psyType: "uint8array", base64: bytesToBase64(value) };
  }
  if (Array.isArray(value)) return value.map(serializeWasmValue);
  if (typeof value === "object") {
    const objectValue = value as {
      toJSON?: () => unknown;
      __wbg_ptr?: unknown;
    };
    if (typeof objectValue.toJSON === "function") {
      return serializeWasmValue(objectValue.toJSON());
    }
    if (value instanceof Map) {
      return Object.fromEntries(
        Array.from(value.entries()).map(([key, item]) => [
          String(key),
          serializeWasmValue(item),
        ]),
      ) as SerializedWasmValue;
    }
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) {
      throw new Error(
        `Cannot serialize non-plain WASM value${objectValue.__wbg_ptr !== undefined ? " with wasm-bindgen pointer" : ""}`,
      );
    }
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [
        key,
        serializeWasmValue(item),
      ]),
    ) as SerializedWasmValue;
  }
  return String(value);
}
