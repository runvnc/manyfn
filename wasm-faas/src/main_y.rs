use std::collections::HashMap;
use std::io;
use std::sync::Mutex;
use std::sync::{Arc, RwLock};
use std::result;

use wasi_common::pipe::WritePipe;
use wasmtime::*;
use wasmtime_wasi::{self, WasiCtx, WasiCtxBuilder};

use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use actix_web::web::{Query, Path};
use actix_web::http::header::ContentType;

struct InstanceData {
    instance: Instance,
    store: Store<WasiCtx>,
}

struct Context {
    stdout_mutex: Arc<RwLock<Vec<u8>>>,
}

struct ModuleCache {
    engine: Engine,
    linker: Linker<WasiCtx>,
    instances: HashMap<String, Arc<Mutex<InstanceData>>>,
}

impl ModuleCache {
    fn new() -> Self {
        let engine = Engine::default();
        let linker = Linker::new(&engine);
        Self {
            engine,
            linker,
            instances: HashMap::new(),
        }
    }

    fn get_instance(&mut self, module_name: &str) -> result::Result<Arc<Mutex<InstanceData>>, anyhow::Error> {
        if let Some(instance) = self.instances.get(module_name) {
            println!("Returning cached instance {}.", module_name);
            Ok(instance.clone())
        } else {
            println!("Not found in cache. Loading instance {}.", module_name);

            wasmtime_wasi::add_to_linker(&mut self.linker, |s| s)?;
            let module = Module::from_file(&self.engine, module_name)?;
            let wasi = WasiCtxBuilder::new().build();
            let mut store = Store::new(&self.engine, wasi);
            let instance = self.linker.instantiate(&mut store, &module)?;

            let instance_data = InstanceData { instance, store };
            let instance_arc = Arc::new(Mutex::new(instance_data));
            self.instances.insert(module_name.to_string(), instance_arc.clone());
            Ok(instance_arc)
        }
    }

    fn create_context(&self, params: HashMap<String, String>) -> result::Result<Context, anyhow::Error> {
        // convert params hashmap to an array
        let envs: Vec<(String, String)> = params
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();

        let stdout_buf: Vec<u8> = vec![];
        let stdout_mutex = Arc::new(RwLock::new(stdout_buf));
        let stdout = WritePipe::from_shared(stdout_mutex.clone());

        let wasi = WasiCtxBuilder::new()
            .stdout(Box::new(stdout))
            .envs(&envs)?
            .build();
        Ok(Context { stdout_mutex })
    }
}

fn invoke_wasm_module(
    module_name: String,
    params: HashMap<String, String>,
    module_cache: &mut ModuleCache,
) -> result::Result<String, anyhow::Error> {
    println!("Loading instance from {}", &module_name);
    let instance_data_arc = module_cache.get_instance(&module_name)?;
    let context = module_cache.create_context(params)?;
    let mut instance_data = instance_data_arc.lock().unwrap();

    let instance_main = instance_data.instance.get_typed_func::<(), (), _>(&instance_data.store, "_start")?;
    instance_main.call(&mut instance_data.store, ())?;

    let buffer: Vec<u8> = context.stdout_mutex.read().unwrap().clone();

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
