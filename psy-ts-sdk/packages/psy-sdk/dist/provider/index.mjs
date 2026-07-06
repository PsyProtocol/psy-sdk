import '../utils/felt.mjs';
import '../utils/json.mjs';
import '../utils/random.mjs';

class RpcConfig {
    constructor(id, rpc_url) {
        this.id = id;
        this.rpc_url = rpc_url;
    }
}

export { RpcConfig };
