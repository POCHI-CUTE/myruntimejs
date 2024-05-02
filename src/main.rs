use anyhow::anyhow;
use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::error::AnyError;
use deno_core::op2;
use deno_core::Extension;
use deno_core::ModuleLoadResponse;
use deno_core::OpDecl;
use deno_core::PollEventLoopOptions;
use reqwest;
use std::env;
use std::rc::Rc;

#[derive(Clone)]
struct SourceMapStore(());

#[op2(async)]
#[string]
async fn op_read_file(#[string] path: String) -> Result<String, AnyError> {
    let contents = tokio::fs::read_to_string(path).await?;
    Ok(contents)
}

#[op2(async)]
#[string]
async fn op_write_file(#[string] path: String, #[string] contents: String) -> Result<(), AnyError> {
    tokio::fs::write(path, contents).await?;
    Ok(())
}

#[op2(fast)]
fn op_remove_file(#[string] path: String) -> Result<(), AnyError> {
    std::fs::remove_file(path)?;
    Ok(())
}

#[op2(async)]
#[string]
async fn op_fetch(#[string] url: String) -> Result<String, AnyError> {
    let body = reqwest::get(url).await?.text().await?;
    Ok(body)
}

struct TsModuleLoader {
    source_maps: SourceMapStore,
}

impl deno_core::ModuleLoader for TsModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: deno_core::ResolutionKind,
    ) -> Result<deno_core::ModuleSpecifier, deno_core::error::AnyError> {
        deno_core::resolve_import(specifier, referrer).map_err(|e| e.into())
    }

    fn load(
        &self,
        module_specifier: &deno_core::ModuleSpecifier,
        _maybe_referrer: Option<&deno_core::ModuleSpecifier>,
        _is_dyn_import: bool,
        _requested_module_type: deno_core::RequestedModuleType,
    ) -> ModuleLoadResponse {
        let source_maps = self.source_maps.clone();
        fn load(
            _source_maps: SourceMapStore,
            module_specifier: &deno_core::ModuleSpecifier,
        ) -> Result<deno_core::ModuleSource, AnyError> {
            let module_specifier = module_specifier.clone();
            let path = module_specifier
                .to_file_path()
                .map_err(|_| anyhow!("Only file:// URLs are supported."))?;

            // Determine what the MediaType is (this is done based on the file
            // extension) and whether transpiling is required.
            let media_type = MediaType::from_path(&path);
            let (module_type, should_transpile) = match MediaType::from_path(&path) {
                MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
                    (deno_core::ModuleType::JavaScript, false)
                }
                MediaType::Jsx => (deno_core::ModuleType::JavaScript, true),
                MediaType::TypeScript
                | MediaType::Mts
                | MediaType::Cts
                | MediaType::Dts
                | MediaType::Dmts
                | MediaType::Dcts
                | MediaType::Tsx => (deno_core::ModuleType::JavaScript, true),
                MediaType::Json => (deno_core::ModuleType::Json, false),
                _ => panic!("Unknown extension {:?}", path.extension()),
            };

            // Read the file, transpile if necessary.
            let code = std::fs::read_to_string(&path)?;
            let code = if should_transpile {
                let parsed = deno_ast::parse_module(ParseParams {
                    specifier: module_specifier.to_string(),
                    text_info: SourceTextInfo::from_string(code),
                    media_type,
                    capture_tokens: false,
                    scope_analysis: false,
                    maybe_syntax: None,
                })?;
                parsed.transpile(&Default::default())?.text
            } else {
                code
            };

            // Load and return module.
            let module = deno_core::ModuleSource::new(
                module_type,
                deno_core::ModuleSourceCode::String(code.into()),
                &module_specifier,
                None,
            );
            Ok(module)
        }
        ModuleLoadResponse::Sync(load(source_maps, module_specifier))
    }
}

async fn run_js(file_path: &str) -> Result<(), AnyError> {
    let current_dir = env::current_dir()?;
    let main_module = deno_core::resolve_path(file_path, &current_dir)?;
    const OP_FETCH_DECL: OpDecl = op_fetch();
    const OP_READ_FILE_DECL: OpDecl = op_read_file();
    const OP_WRITE_FILE_DECL: OpDecl = op_write_file();
    const OP_REMOVE_FILE_DECL: OpDecl = op_remove_file();
    let runjs_extension = Extension {
        name: "runjs",
        ops: std::borrow::Cow::Borrowed(&[
            OP_FETCH_DECL,
            OP_READ_FILE_DECL,
            OP_WRITE_FILE_DECL,
            OP_REMOVE_FILE_DECL,
        ]),
        ..Default::default()
    };
    let source_map_store = SourceMapStore(());
    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        // module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        module_loader: Some(Rc::new(TsModuleLoader {
            source_maps: source_map_store.clone(),
        })),
        extensions: vec![runjs_extension],
        ..Default::default()
    });
    js_runtime
        .execute_script("[runjs:runtime.js]", include_str!("./runtime.js"))
        .unwrap();

    let mod_id = js_runtime.load_main_es_module(&main_module).await?;
    let result = js_runtime.mod_evaluate(mod_id);
    js_runtime
        .run_event_loop(PollEventLoopOptions::default())
        .await?;
    result.await?;
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.is_empty() {
        eprintln!("Please provide a file path to run");
        std::process::exit(1);
    }
    let file_path = &args[1];

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    if let Err(error) = runtime.block_on(run_js(file_path)) {
        eprintln!("error: {}", error);
    }
}
