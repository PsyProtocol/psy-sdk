'use strict';

Object.defineProperty(exports, '__esModule', { value: true });

var _documentCurrentScript = typeof document !== 'undefined' ? document.currentScript : null;
/* @ts-self-types="./psy_compiler.d.ts" */
/**
 * @param {bigint} caller_id
 * @param {bigint} contract_id
 * @param {string} method_name
 * @param {string} args_json
 * @returns {string}
 */
function call_contract(caller_id, contract_id, method_name, args_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(method_name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(args_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.call_contract(caller_id, contract_id, ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}
/**
 * @param {string} project_json
 * @returns {string}
 */
function compile_dargo_project(project_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(project_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.compile_dargo_project(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}
/**
 * @param {string} files_json
 * @returns {string}
 */
function compile_project(files_json) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(files_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.compile_project(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}
/**
 * @param {string} source
 * @returns {string}
 */
function compile_source(source) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.compile_source(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}
/**
 * @param {string} name
 * @returns {string}
 */
function create_account(name) {
    let deferred2_0;
    let deferred2_1;
    try {
        const ptr0 = passStringToWasm0(name, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.create_account(ptr0, len0);
        deferred2_0 = ret[0];
        deferred2_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred2_0, deferred2_1, 1);
    }
}
/**
 * @param {bigint} deployer_id
 * @returns {string}
 */
function deploy_contract(deployer_id) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.deploy_contract(deployer_id);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
/**
 * @returns {string}
 */
function get_accounts() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_accounts();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
/**
 * @param {bigint} contract_id
 * @returns {string}
 */
function get_contract_abi(contract_id) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_contract_abi(contract_id);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
/**
 * @returns {string}
 */
function get_contracts() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_contracts();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
/**
 * @returns {string}
 */
function get_transaction_log() {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.get_transaction_log();
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
function init_chain() {
    wasm.init_chain();
}
function init_logging() {
    wasm.init_logging();
}
function init_psy_ide() {
    wasm.init_psy_ide();
}
/**
 * @param {string} files_json
 * @param {string} request_json
 * @returns {string}
 */
function interpret_project(files_json, request_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(files_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.interpret_project(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}
/**
 * @param {string} source
 * @param {string} request_json
 * @returns {string}
 */
function interpret_source(source, request_json) {
    let deferred3_0;
    let deferred3_1;
    try {
        const ptr0 = passStringToWasm0(source, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ptr1 = passStringToWasm0(request_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len1 = WASM_VECTOR_LEN;
        const ret = wasm.interpret_source(ptr0, len0, ptr1, len1);
        deferred3_0 = ret[0];
        deferred3_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred3_0, deferred3_1, 1);
    }
}
function main() {
    wasm.main();
}
/**
 * @param {bigint} contract_id
 * @param {bigint} user_id
 * @returns {string}
 */
function read_contract_state(contract_id, user_id) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.read_contract_state(contract_id, user_id);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
/**
 * @param {number} contract_id
 * @param {number} user_id
 * @returns {string}
 */
function read_imt_state(contract_id, user_id) {
    let deferred1_0;
    let deferred1_1;
    try {
        const ret = wasm.read_imt_state(contract_id, user_id);
        deferred1_0 = ret[0];
        deferred1_1 = ret[1];
        return getStringFromWasm0(ret[0], ret[1]);
    }
    finally {
        wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
    }
}
function reset_chain() {
    wasm.reset_chain();
}
function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_ea4887a5f8f9a9db: function (arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_debug_7271beced8b71cd4: function (arg0, arg1, arg2, arg3) {
            console.debug(arg0, arg1, arg2, arg3);
        },
        __wbg_error_50f60c611a3dcf64: function (arg0, arg1, arg2, arg3) {
            console.error(arg0, arg1, arg2, arg3);
        },
        __wbg_error_933f449d72fef598: function (arg0) {
            console.error(arg0);
        },
        __wbg_error_a6fa202b58aa1cd3: function (arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            }
            finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_info_a392cd5b7536cfb5: function (arg0, arg1, arg2, arg3) {
            console.info(arg0, arg1, arg2, arg3);
        },
        __wbg_log_17a3e9a5cbb91ef7: function (arg0, arg1, arg2, arg3, arg4, arg5, arg6, arg7) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.log(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3), getStringFromWasm0(arg4, arg5), getStringFromWasm0(arg6, arg7));
            }
            finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_log_d282446d03691e72: function (arg0, arg1, arg2, arg3) {
            console.log(arg0, arg1, arg2, arg3);
        },
        __wbg_log_e885b89e7e480a2f: function (arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.log(getStringFromWasm0(arg0, arg1));
            }
            finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_mark_0279c5d75168b5b8: function (arg0, arg1) {
            performance.mark(getStringFromWasm0(arg0, arg1));
        },
        __wbg_measure_c9b58ac538b3e2f7: function () {
            return handleError(function (arg0, arg1, arg2, arg3) {
                let deferred0_0;
                let deferred0_1;
                let deferred1_0;
                let deferred1_1;
                try {
                    deferred0_0 = arg0;
                    deferred0_1 = arg1;
                    deferred1_0 = arg2;
                    deferred1_1 = arg3;
                    performance.measure(getStringFromWasm0(arg0, arg1), getStringFromWasm0(arg2, arg3));
                }
                finally {
                    wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
                    wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
                }
            }, arguments);
        },
        __wbg_new_227d7c05414eb861: function () {
            const ret = new Error();
            return ret;
        },
        __wbg_stack_3b0d974bbf31e44f: function (arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_warn_88c4a5bd9a322000: function (arg0, arg1, arg2, arg3) {
            console.warn(arg0, arg1, arg2, arg3);
        },
        __wbindgen_cast_0000000000000001: function (arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function () {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./psy_compiler_bg.js": import0,
    };
}
function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}
let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}
function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}
let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}
function handleError(f, args) {
    try {
        return f.apply(this, args);
    }
    catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}
function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }
    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;
    const mem = getUint8ArrayMemory0();
    let offset = 0;
    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F)
            break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);
        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }
    WASM_VECTOR_LEN = offset;
    return ptr;
}
let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}
const cachedTextEncoder = new TextEncoder();
if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}
let WASM_VECTOR_LEN = 0;
let wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    cachedDataViewMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}
async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            }
            catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);
                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);
                }
                else {
                    throw e;
                }
            }
        }
        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    }
    else {
        const instance = await WebAssembly.instantiate(module, imports);
        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        }
        else {
            return instance;
        }
    }
    function expectedResponseType(type) {
        switch (type) {
            case 'basic':
            case 'cors':
            case 'default': return true;
        }
        return false;
    }
}
function initSync(module) {
    if (wasm !== undefined)
        return wasm;
    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({ module } = module);
        }
        else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead');
        }
    }
    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance);
}
async function __wbg_init(module_or_path) {
    if (wasm !== undefined)
        return wasm;
    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({ module_or_path } = module_or_path);
        }
        else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead');
        }
    }
    if (module_or_path === undefined) {
        module_or_path = new URL('psy_compiler_bg.wasm', (typeof document === 'undefined' ? require('u' + 'rl').pathToFileURL(__filename).href : (_documentCurrentScript && _documentCurrentScript.tagName.toUpperCase() === 'SCRIPT' && _documentCurrentScript.src || new URL('local-web-compiler/psy_compiler.cjs', document.baseURI).href)));
    }
    const imports = __wbg_get_imports();
    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }
    const { instance, module } = await __wbg_load(await module_or_path, imports);
    return __wbg_finalize_init(instance);
}

exports.call_contract = call_contract;
exports.compile_dargo_project = compile_dargo_project;
exports.compile_project = compile_project;
exports.compile_source = compile_source;
exports.create_account = create_account;
exports.default = __wbg_init;
exports.deploy_contract = deploy_contract;
exports.get_accounts = get_accounts;
exports.get_contract_abi = get_contract_abi;
exports.get_contracts = get_contracts;
exports.get_transaction_log = get_transaction_log;
exports.initSync = initSync;
exports.init_chain = init_chain;
exports.init_logging = init_logging;
exports.init_psy_ide = init_psy_ide;
exports.interpret_project = interpret_project;
exports.interpret_source = interpret_source;
exports.main = main;
exports.read_contract_state = read_contract_state;
exports.read_imt_state = read_imt_state;
exports.reset_chain = reset_chain;
