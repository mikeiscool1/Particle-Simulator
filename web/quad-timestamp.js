var ctx = null;
var memory;

params_set_mem = function (wasm_memory, _wasm_exports) {
    memory = wasm_memory;
    ctx = {};
}
 

params_register_js_plugin = function (importObject) {

    importObject.env.timestamp_utc = function () { 
        return Math.round(new Date().getTime()/1000);
    } 

    importObject.env.timestamp_utc_ms = function () {
        return Math.round(new Date().getTime());
    } 
}

miniquad_add_plugin({
    register_plugin: params_register_js_plugin,
    on_init: params_set_mem,
    name: "quad_timestamp",
    version: "0.1.2"
});
