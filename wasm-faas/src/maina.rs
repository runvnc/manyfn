use std::collections::HashMap;
use std::sync::RwLock;
use once_cell::sync::Lazy;
use wasmtime::*;
use wasmtime_wasi::sync::WasiCtxBuilder;

// Initialize the engine.
static ENGINE: Lazy<Engine> = Lazy::new(|| Engine::default());

// Define a struct for holding tenant data.
struct TenantData {
    store: Store<()>,
    modules: HashMap<String, Module>,
}

// Use a HashMap for storing tenant data, keyed by tenant ID.
// The RwLock allows for concurrent reads, which is useful if you're serving multiple requests at once.
static TENANT_DATA: Lazy<RwLock<HashMap<String, TenantData>>> = Lazy::new(|| RwLock::new(HashMap::new()));

// Function for getting a module, compiling it if necessary.
// This would go inside your request handling code, where you know the tenant_id and module_name.
// Use Arc and Mutex to handle shared access and modification across threads
fn get_module(tenant_id: &str, module_name: &str) -> Result<Module, Box<dyn std::error::Error>> {
    // Lock the TENANT_DATA for writing.
    let mut tenant_data_map = TENANT_DATA.write().unwrap();

    // Get the TenantData for the tenant, or insert a new one if it doesn't exist.
    let tenant_data = tenant_data_map.entry(tenant_id.to_string()).or_insert_with(|| TenantData {
        store: Store::new(&*ENGINE, ()),
        modules: HashMap::new(),
    });

    // Get the Module from the tenant's cache, or insert a new one if it doesn't exist.
    let module = tenant_data.modules.entry(module_name.to_string()).or_insert_with(|| {
        let fname = module_name.to_owned()+".wasm";
        println!("Loading module from file: {}", fname);
        let wasm = std::fs::read(fname).unwrap();
        Module::new(&*ENGINE, &wasm).unwrap()
    });

    Ok(module.clone())  // Clone the Module for use in the caller.
}

use wasmtime::Val;

fn call_module_function(tenant_id: &str, module_name: &str, function_name: &str, msg: &str) 
    -> Result<Box<[Val]>, Box<dyn std::error::Error>> {
    // Get the module.
    let module = get_module(tenant_id, module_name)?;

    // Get a reference to the tenant data map.
    let tenant_data_map = TENANT_DATA.read().unwrap();
    
    // Get a reference to the Store for the tenant.
    let tenant_data = tenant_data_map.get(tenant_id).unwrap();
    let store = &tenant_data.store;
    
    // Set up a new Linker with a WasiCtx.
    let wasi_ctx = WasiCtxBuilder::new().inherit_stdio().build();
    let mut linker = Linker::new(&*ENGINE);
    //wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
     
    
    // Instantiate the module and get the function.
    //let instance = linker.instantiate(&module)?;
    //let function = instance.get_func(function_name).ok_or("function not found")?;
    
    // Call the function with the message as a parameter.
    //let msg_val = Val::String(msg.to_string());
    //let results = function.call(msg_val)?;
    let results = Box::new([Val::I32(11)]);
    Ok(results) 
} 

fn main() {
  println!("Hi");
  let res = call_module_function("bob", "hello", "func", "heyy").unwrap();
  println!("done")
}

