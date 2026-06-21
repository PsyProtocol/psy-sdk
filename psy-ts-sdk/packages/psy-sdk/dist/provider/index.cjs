'use strict';

require('../utils/felt.cjs');
require('../utils/json.cjs');
require('../utils/random.cjs');

class RpcConfig {
    constructor(id, rpc_url) {
        this.id = id;
        this.rpc_url = rpc_url;
    }
}

exports.RpcConfig = RpcConfig;
