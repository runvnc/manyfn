use std::collections::HashMap;
use std::io;
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::result;

use wasi_common::pipe::WritePipe;
use wasmtime::*;
use wasmtime_wasi::{self, WasiCtxBuilder};


use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use actix_web::web::{Query, Path};
use actix_web::http::header::ContentType;


struct ModuleCache {
    modules: HashMap<String, Arc<Module>>,
}

impl ModuleCache {
    fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    fn get_module(&mut self, engine: &Engine, module_name: &str) -> result::Result<Arc<Module>, anyhow::Error> {

        if let Some(module) = self.modules.get(module_name) {
            println!("Returning cached module {}.", module_name);
            Ok(module.clone())
        } else {
            println!("Not found in cache. Loading module {}.", module_name);
            let module = Module::from_file(&engine, module_name)?;
            let module_arc = Arc::new(module.clone());
            self.modules.insert(module_name.to_string(), module_arc.clone());
            Ok(module_arc)
        }
    }
}

fn invoke_wasm_module(
    module_name: String,
    params: HashMap<String, String>,
    module_cache: &mut ModuleCache,
) -> result::Result<String, anyhow::Error> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;

    let stdout_buf: Vec<u8> = vec![];
    let stdout_mutex = Arc::new(RwLock::new(stdout_buf));
    let stdout = WritePipe::from_shared(stdout_mutex.clone());

    // convert params hashmap to an array
    let envs: Vec<(String, String)> = params
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();

    let wasi = WasiCtxBuilder::new()
        .stdout(Box::new(stdout))
        .envs(&envs)?
        .build();
    let mut store = Store::new(&engine, wasi);

    println!("Loading module from {}", &module_name);

    let module = module_cache.get_module(&engine, &module_name)?;
    linker.module(&mut store, &module_name, &module)?;

    let instance = linker.instantiate(&mut store, &module)?;
    let instance_main = instance.get_typed_func::<(), (), _>(&mut store, "_start")?;
    instance_main.call(&mut store, ())?;

    let mut buffer: Vec<u8> = Vec::new();
    stdout_mutex.read().unwrap().iter().for_each(|i| {
        buffer.push(*i)
    });

    let s = String::from_utf8(buffer)?;
    Ok(s)
}

#[get("/favicon.ico")]
async fn favicon_handler() -> HttpResponse {
    HttpResponse::NotFound().finish()
}

#[get("/{module_name}")]
async fn handler(module_name: Path<String>,
                 query: Query<HashMap<String, String>>,
                 module_cache: actix_web::web::Data<Mutex<ModuleCache>>)
    -> impl Responder {
      let wasm_module = format!("{}{}{}", "api/",module_name, ".wasm");  
      let val = invoke_wasm_module(wasm_module, query.into_inner(),
                                 &mut *module_cache.lock().unwrap()).expect("invocation error");
      HttpResponse::Ok().insert_header(ContentType::plaintext()).body(val)
}


#[actix_web::main]
async fn main() -> io::Result<()> {
    println!("Server starting on port 8288.");
    let module_cache = actix_web::web::Data::new(Mutex::new(ModuleCache::new()));

    HttpServer::new(move || {
            App::new()
            .app_data(module_cache.clone())
            .service(favicon_handler)
            .service(handler)
        })
        .bind("0.0.0.0:8288")?
        .run()
        .await
}
