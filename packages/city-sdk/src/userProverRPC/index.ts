import { FetchHTTPClient } from "../http/fetchClient";
import { ICityHTTPClient } from "../http/types";
import { waitMs } from "../utils/time";
import { CityUserProverRPCCommand, ICityUserProverProvider } from "./types";

class CityRPCUserProverProvider implements ICityUserProverProvider {
  httpClient: ICityHTTPClient;
  url: string;
  constructor(url: string, httpClient?: ICityHTTPClient) {
    this.httpClient = httpClient || (new FetchHTTPClient());
    this.url = url;
  }

  async rpc<T>(method: string, params: any[], id = "1", jsonrpc = "2.0"): Promise<T> {
    const result = await this.httpClient.sendRequest({
      method: "POST",
      url: this.url,
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        jsonrpc,
        method,
        params,
        id,
      }),
      responseType: "json",
    });
    if (result.statusCode >= 400) {
      throw new Error("Error in RPC call: " + JSON.stringify(result.body));
    } else {
      return result.body.result as T;
    }
  }
  async getResultFinal(hash: Promise<string>, maxAttempts: number, delay: number) {
    const resolvedHash = await hash;
    for (let i = 0; i < maxAttempts; i++) {
      try {
        const result = await this.getResult(resolvedHash);
        return result;
      } catch (e) {
      }
      await waitMs(delay);
    }
    throw new Error("Result not found after " + maxAttempts + " attempts");
  }
  proveSecp256K1Signature(publicKey: string, signature: string, message: string, maxAttempts = 120, delay = 500): Promise<string> {
    return this.getResultFinal(this.rpc<string>(CityUserProverRPCCommand.ProveSecp256K1Signature, [publicKey, signature, message]), maxAttempts, delay);
  }
  proveZKSignature(privateKey: string, message: string, maxAttempts = 120, delay = 500): Promise<string> {
    return this.getResultFinal(this.rpc<string>(CityUserProverRPCCommand.ProveZKSignature, [privateKey, message]), maxAttempts, delay);
  }
  proveZKSignatureEnc(encryptedPrivateKey: string, message: string, salt: string, maxAttempts = 120, delay = 500): Promise<string> {
    return this.getResultFinal(this.rpc<string>(CityUserProverRPCCommand.ProveZKSignatureEnc, [encryptedPrivateKey, message, salt]), maxAttempts, delay);
  }
  getZKPublicKey(privateKey: string, maxAttempts = 120, delay = 500): Promise<string> {
    return this.getResultFinal(this.rpc<string>(CityUserProverRPCCommand.GetZKPublicKey, [privateKey]), maxAttempts, delay);
  }
  getZKPublicKeyEnc(encryptedPrivateKey: string, salt: string, maxAttempts = 120, delay = 500): Promise<string> {
    return this.getResultFinal(this.rpc<string>(CityUserProverRPCCommand.GetZKPublicKeyEnc, [encryptedPrivateKey, salt]), maxAttempts, delay);
  }

  proveSecp256K1SignatureBase(publicKey: string, signature: string, message: string): Promise<string> {
    return this.rpc<string>(CityUserProverRPCCommand.ProveSecp256K1Signature, [publicKey, signature, message]);
  }
  proveZKSignatureBase(privateKey: string, message: string): Promise<string> {
    return this.rpc<string>(CityUserProverRPCCommand.ProveZKSignature, [privateKey, message]);
  }
  proveZKSignatureEncBase(encryptedPrivateKey: string, message: string, salt: string): Promise<string> {
    return this.rpc<string>(CityUserProverRPCCommand.ProveZKSignatureEnc, [encryptedPrivateKey, message, salt]);
  }
  getZKPublicKeyBase(privateKey: string): Promise<string> {
    return this.rpc<string>(CityUserProverRPCCommand.GetZKPublicKey, [privateKey]);
  }
  getZKPublicKeyEncBase(encryptedPrivateKey: string, salt: string): Promise<string> {
    return this.rpc<string>(CityUserProverRPCCommand.GetZKPublicKeyEnc, [encryptedPrivateKey, salt]);
  }
  getResult(hash: string): Promise<string> {
    return this.rpc<string>(CityUserProverRPCCommand.GetResult, [hash]);
  }

  zkSignHash(privateKey: string, message: string): Promise<string> {
    return this.proveZKSignature(privateKey, message);
  }
  generateSecp256K1SignatureProof(publicKey: string, signature: string, message: string): Promise<string> {
    return this.proveSecp256K1Signature(publicKey, signature, message);
  }

  getZKPublicKeyForPrivateKey(privateKey: string): Promise<string> {
    return this.getZKPublicKey(privateKey);
  }
}

export {
  CityRPCUserProverProvider,
};