export * from "./provider";

export * from "./baseProvider";

export class RpcConfig {
    id: number;
    rpc_url: string[];

    constructor(id: number, rpc_url: string[]) {
        this.id = id;
        this.rpc_url = rpc_url;
    }
}
