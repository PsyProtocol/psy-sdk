import { compile_project, compile_source, interpret_project, interpret_source, initSync } from './psy_compiler.mjs';
import { wasmBinary } from './wasm-binary.mjs';
import '../utils/felt.mjs';
import { PsyJSON } from '../utils/json.mjs';
import '../utils/random.mjs';

let initialized = false;
function ensureInit() {
    if (!initialized) {
        initSync({ module: wasmBinary });
        initialized = true;
    }
}
/**
 * Compile a single PSY source file in-browser.
 * @param source - PSY source code string
 */
function compileSource(source) {
    ensureInit();
    return PsyJSON.parse(compile_source(source));
}
/**
 * Compile a multi-file PSY project in-browser.
 * @param input - Explicit project input.
 *   e.g. { entry: ["main"], method_names: ["main"], files: [[["main"], "mod foo;\nfn main() {}"], [["foo"], "pub fn run() {}"]] }
 */
function compileProject(input) {
    ensureInit();
    return PsyJSON.parse(compile_project(PsyJSON.stringify(input)));
}
function interpretSource(source, request) {
    ensureInit();
    return PsyJSON.parse(interpret_source(source, PsyJSON.stringify(request)));
}
function interpretProject(input, request) {
    ensureInit();
    return PsyJSON.parse(interpret_project(PsyJSON.stringify(input), PsyJSON.stringify(request)));
}

export { compileProject, compileSource, interpretProject, interpretSource };
