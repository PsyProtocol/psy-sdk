'use strict';

var localWebCompiler_psy_compiler = require('./psy_compiler.cjs');
var localWebCompiler_wasmBinary = require('./wasm-binary.cjs');
require('../utils/felt.cjs');
var json = require('../utils/json.cjs');
require('../utils/random.cjs');

let initialized = false;
function ensureInit() {
    if (!initialized) {
        localWebCompiler_psy_compiler.initSync({ module: localWebCompiler_wasmBinary.wasmBinary });
        initialized = true;
    }
}
/**
 * Compile a single PSY source file in-browser.
 * @param source - PSY source code string
 */
function compileSource(source) {
    ensureInit();
    return json.PsyJSON.parse(localWebCompiler_psy_compiler.compile_source(source));
}
/**
 * Compile a multi-file PSY project in-browser.
 * @param input - Explicit project input.
 *   e.g. { entry: ["main"], method_names: ["main"], files: [[["main"], "mod foo;\nfn main() {}"], [["foo"], "pub fn run() {}"]] }
 */
function compileProject(input) {
    ensureInit();
    return json.PsyJSON.parse(localWebCompiler_psy_compiler.compile_project(json.PsyJSON.stringify(input)));
}
function interpretSource(source, request) {
    ensureInit();
    return json.PsyJSON.parse(localWebCompiler_psy_compiler.interpret_source(source, json.PsyJSON.stringify(request)));
}
function interpretProject(input, request) {
    ensureInit();
    return json.PsyJSON.parse(localWebCompiler_psy_compiler.interpret_project(json.PsyJSON.stringify(input), json.PsyJSON.stringify(request)));
}

exports.compileProject = compileProject;
exports.compileSource = compileSource;
exports.interpretProject = interpretProject;
exports.interpretSource = interpretSource;
