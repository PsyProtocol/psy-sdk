import { fetchHTTPClient } from "../helpers/http";
import { IDogeHTTPClient, ISimpleHTTPRequest } from "../types/http";
import { IDogeLinkRPCInfo } from "../types/network";
import { seq } from "../utils/core";
import { getNetworkById } from "../utils/networks";
import { parseDogeLinkNetworkURI } from "../utils/parseNetwork";
import { Block } from "bitcoinjs-lib";

class DogeLinkRPC {
  rpcInfo: IDogeLinkRPCInfo;
  httpClient: IDogeHTTPClient;

  constructor(
    rpcInfo: IDogeLinkRPCInfo | string,
    httpClient: IDogeHTTPClient = fetchHTTPClient
  ) {
    if (typeof rpcInfo === "string") {
      this.rpcInfo = parseDogeLinkNetworkURI(rpcInfo);
    } else {
      this.rpcInfo = rpcInfo;
    }
    this.httpClient = httpClient;
  }
  getNetwork() {
    return getNetworkById(this.rpcInfo.network);
  }
  command<T = any>(
    method: string,
    params: any,
    version = "1.0",
    path = ""
  ): Promise<T> {
    const request: ISimpleHTTPRequest = {
      url: this.rpcInfo.url + path,
      method: "POST",
      credentials: "include",
      headers:
        this.rpcInfo.user && this.rpcInfo.password
          ? {
              "Content-Type": "application/json",
              Authorization:
                "Basic " +
                btoa(this.rpcInfo.user + ":" + this.rpcInfo.password),
            }
          : {
              "Content-Type": "application/json",
            },
      body: JSON.stringify({
        jsonrpc: version,
        method,
        params,
        id: 1,
      }),
      responseType: "json",
    };
    return this.httpClient.sendRequest(request).then(x=>{
      if(x.body.error){
        throw new Error(x.body.error.message || "unknown error");
      }else{
        return x.body.result;
      }
    })
  }

  getBlockCount(): Promise<number> {
    return this.command<number>("getblockcount", []);
  }
  getRawTransaction(txId: string): Promise<string> {
    return this.command<string>("getrawtransaction", [txId]);
  }
  getBlockHash(height: number): Promise<string> {
    return this.command<string>("getblockhash", [height]);
  }
  getWalletAddress(walletName = "default") {
    if (this.isDoge()) {
      return this.command<string>("getnewaddress", [], "1.0");
    } else {
      return this.command<string>(
        "getnewaddress",
        [],
        "1.0",
        "wallet/" + encodeURIComponent(walletName)
      );
    }
  }
  mineBlocks(count: number, address = "") {
    if (this.rpcInfo.network === "dogeRegtest") {
      const rAddress = address
        ? Promise.resolve(address)
        : this.getWalletAddress();
      return rAddress.then((address) =>
        this.command<string[]>("generatetoaddress", [count, address])
      );
    } else {
      // disable mine blocks for non regtest networks
      return Promise.resolve(
        "0000000000000000000000000000000000000000000000000000000000000000"
      );
    }
  }
  isDoge() {
    return (
      this.rpcInfo.network === "doge" ||
      this.rpcInfo.network === "dogeRegtest" ||
      this.rpcInfo.network === "dogeTestnet"
    );
  }
  sendFromWallet(
    address: string,
    amount: number | string,
    walletName: string = "default"
  ) {
    if (this.isDoge()) {
      // old api
      return this.command<string>(
        "sendtoaddress",
        [address, amount, "", "", true],
        "1.0"
      );
    } else {
      // bitcoin core (latest)
      return this.command<string>(
        "sendtoaddress",
        [address, amount],
        "1.0",
        "wallet/" + encodeURIComponent(walletName)
      );
    }
  }
  sendRawTransaction(txHex: string) {
    return this.command<string>("sendrawtransaction", [txHex]);
  }

  getBlock(blockHashOrNumber: string | number): Promise<Block> {
    const baseHashResult =
      typeof blockHashOrNumber === "string"
        ? Promise.resolve(blockHashOrNumber)
        : this.getBlockHash(blockHashOrNumber);

    return baseHashResult
      .then((r) => this.command("getblock", [r, 0]))
      .then((x) => Block.fromHex(x));
  }

  getBlockExtra(blockHashOrNumber: string | number): Promise<any> {
    const baseHashResult =
      typeof blockHashOrNumber === "string"
        ? Promise.resolve(blockHashOrNumber)
        : this.getBlockHash(blockHashOrNumber);

    return baseHashResult
      .then((r) => this.command("getblock", [r, 2]))
      .then((x) => Block.fromHex(x));
  }

  getBlocks(start: number, count: number): Promise<Block[]> {
    return Promise.all(
      seq(count, start).map((x) =>
        this.getBlockHash(x)
          .then((hash) => this.command("getblock", [hash, 0]))
          .then((x) => Block.fromHex(x))
      )
    );
  }
  resolveBlockHash(blockHashOrNumber: string | number): Promise<string> {
    if (typeof blockHashOrNumber === "number") {
      return this.getBlockHash(blockHashOrNumber);
    } else {
      return Promise.resolve(blockHashOrNumber);
    }
  }
  resolveBlockNumber(blockHashOrNumber: string | number): Promise<number> {
    if (typeof blockHashOrNumber === "string") {
      return this.getBlockExtra(blockHashOrNumber).then((x) => x.height);
    } else {
      return Promise.resolve(blockHashOrNumber);
    }
  }
}

export { DogeLinkRPC };
