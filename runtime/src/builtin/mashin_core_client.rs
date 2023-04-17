use crate::{
    fetch_once,
    http_client::{self, HttpClient},
    js_log, log, FetchOnceArgs, FetchOnceResult,
};
use deno_core::{
    error::{bad_resource_id, type_error, AnyError},
    resolve_path,
    serde_json::{self, Value},
    ModuleSpecifier, OpState, Resource, ResourceId,
};
use libffi::middle::Arg;
use mashin_core::{
    colors,
    sdk::{
        ext::anyhow::{anyhow, bail},
        ResourceAction, ResourceArgs, ResourceDiff, ResourceResult, Result, Urn,
    },
    BackendState, ExecutedResource, FileState, MashinEngine, ProviderInner, RawState,
    RegisteredProvider,
};
use std::{
    alloc::{dealloc, Layout},
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    env::{self, current_dir},
    ffi::c_void,
    path::PathBuf,
    ptr,
    rc::Rc,
    str::FromStr,
    sync::mpsc::{self, TryRecvError},
    thread::{self, sleep},
    time::{Duration, Instant},
};
use tokio::runtime::{Handle, Runtime};

use deno_core::error::generic_error;
use dlopen::raw::Library;
use mashin_core::{DynamicLibraryResource, ForeignFunction, NativeType, NativeValue, Symbol};
use serde::Deserialize;

// only call if we want to overwrite the backend
#[deno_core::op]
pub(crate) async fn as__client_new(
    op_state: Rc<RefCell<OpState>>,
    backend_rid: Option<ResourceId>,
) -> Result<()> {
    Ok(())
}

#[derive(Deserialize, Debug)]
pub struct ResourceExecuteArgs {
    urn: String,
    config: serde_json::Value,
}

#[deno_core::op]
pub(crate) fn as__runtime__resource_execute(
    op_state: &mut OpState,
    args: ResourceExecuteArgs,
) -> Result<serde_json::Value> {
    let mashin = op_state.borrow_mut::<MashinEngine>();
    let mut executed_resouces = mashin.executed_resources.borrow_mut();

    // resource config
    let raw_config = Rc::new(args.config);

    // the URN of the resource
    // urn:mashin:aws:s3:bucket/?=mysuper_bucket
    // q_component is the resource name
    let urn = Rc::new(Urn::from_str(&args.urn)?);
    let provider_name = urn.as_provider()?;
    let display_urn = urn.as_display();

    let already_executed_resource = executed_resouces.get(&urn);
    let expected_resource_action =
        if let Some(already_executed_resource) = already_executed_resource {
            already_executed_resource
                .required_change
                .clone()
                .unwrap_or(ResourceAction::Get)
        } else {
            ResourceAction::Get
        };

    let start = Instant::now();
    let present_participe_action = expected_resource_action
        .action_present_participe_str()
        .to_string();

    log!(
        info,
        "[{}]: {}...",
        colors::bold(&display_urn),
        &present_participe_action,
    );

    let backend = mashin.state_handler.borrow();
    let providers = mashin.providers.borrow();

    let provider = providers
        .get(&provider_name)
        .ok_or(anyhow!("provider initialized"))?;

    let raw_state = Rc::new(RefCell::new(
        backend
            .get(&urn)?
            .map(|s| s.decrypt(&mashin.key))
            .map_or(Ok(None), |v| v.map(Some))?
            .unwrap_or_default()
            .inner()
            .clone(),
    ));

    let args_pointer = Rc::into_raw(Rc::new(ResourceArgs {
        action: Rc::new(expected_resource_action.clone()),
        raw_config,
        raw_state: raw_state.clone(),
        urn: urn.clone(),
    }));

    // launch a new thread to display the log if it take more than 5 seconds
    // eg; aws:s3:bucket?=test1234atmos001: Refreshing... 10s
    let (tx, rx) = mpsc::channel();
    let isolated_urn = colors::bold(display_urn.clone());
    thread::spawn(move || loop {
        sleep(Duration::from_secs(5));

        match rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => {
                break;
            }
            Err(TryRecvError::Empty) => {}
        };

        log!(
            info,
            "[{isolated_urn}]: {}... {}s",
            present_participe_action.clone(),
            start.elapsed().as_secs()
        );
    });

    // call the function
    let provider_state = provider
        .dylib
        .call_resource(provider.ptr, args_pointer as *mut c_void)?;

    // close the log thread
    tx.send(())?;

    // grab the raw state from the provider job run previously
    let new_state = unsafe {
        let state = Rc::from_raw(provider_state);
        state.inner().into()
    };
    // take the current state to compare with the new one
    let current_state = raw_state.as_ref().take().into();

    // this is the first run
    if already_executed_resource.is_none() {
        let executed_resource = ExecutedResource::new(
            provider_name,
            //args,
            &current_state,
            &new_state,
        );

        executed_resouces.insert(&urn, executed_resource);
    } else {
        executed_resouces.remove(&urn);
    }

    log!(
        info,
        "[{}]: {} complete after {}s",
        colors::bold(&display_urn),
        expected_resource_action.action_present_str(),
        start.elapsed().as_secs()
    );

    Ok(new_state.generate_ts_output())
}

#[derive(Default, Deserialize, Debug)]
pub enum ProviderDownloadSource {
    #[default]
    #[serde(rename(deserialize = "github"))]
    GithubRelease,
}

#[derive(Deserialize, Debug)]
pub struct ProviderDownloadArgs {
    provider: ProviderDownloadSource,
    url: String,
}

#[deno_core::op]
pub async fn as__runtime__register_provider__download(
    op_state_rc: Rc<RefCell<OpState>>,
    args: ProviderDownloadArgs,
) -> Result<String> {
    let provider = &args.provider;
    let remote_url = &args.url;
    let op_state = op_state_rc.borrow();
    let module_specifier = ModuleSpecifier::from_str(remote_url)?;
    let http_client = op_state.borrow::<HttpClient>();

    let cached_local_path = match provider {
        ProviderDownloadSource::GithubRelease => {
            match http_client.cache().fetch_cached_path(&module_specifier, 10) {
                Ok(Some(cache_filename)) => cache_filename.into_os_string().into_string(),
                Ok(None) => {
                    let (remote_data, headers) = http_client
                        .download_with_headers(module_specifier.clone())
                        .await?;
                    let file = http_client
                        .cache()
                        .set(&module_specifier, headers, &remote_data)?;
                    file.into_os_string().into_string()
                }
                Err(err) => return Err(err),
            }
        }
    };

    cached_local_path.map_err(|_| anyhow!("Something went wrong with provider cdylib path"))
}

#[derive(Deserialize, Debug)]
pub struct ProviderAllocateArgs {
    name: String,
    path: String,
    symbols: HashMap<String, ForeignFunction>,
    props: Value,
}

#[deno_core::op]
pub fn as__runtime__register_provider__allocate(
    op_state: &mut OpState,
    args: ProviderAllocateArgs,
) -> Result<()> {
    println!("INIT PROVIDER");
    let path = args.path;
    let provider_name = args.name;
    let props = args.props;

    let mashin = op_state.borrow_mut::<MashinEngine>();
    let mut providers = mashin.providers.borrow_mut();

    let lib = Library::open(&path).map_err(|e| {
        dlopen::Error::OpeningLibraryError(std::io::Error::new(
            std::io::ErrorKind::Other,
            super::ffi::format_error(e, path),
        ))
    })?;
    let mut resource = DynamicLibraryResource {
        lib,
        symbols: HashMap::new(),
    };

    for (symbol_key, foreign_fn) in args.symbols {
        let symbol = match &foreign_fn.name {
            Some(symbol) => symbol,
            None => &symbol_key,
        };
        // By default, Err returned by this function does not tell
        // which symbol wasn't exported. So we'll modify the error
        // message to include the name of symbol.
        let fn_ptr =
        // SAFETY: The obtained T symbol is the size of a pointer.
        match unsafe { resource.lib.symbol::<*const c_void>(symbol) } {
            Ok(value) => Ok(value),
            Err(err) => Err(generic_error(format!(
            "Failed to register symbol {symbol}: {err}"
            ))),
        }?;
        let ptr = libffi::middle::CodePtr::from_ptr(fn_ptr as _);
        let cif = libffi::middle::Cif::new(
            foreign_fn
                .parameters
                .clone()
                .into_iter()
                .map(libffi::middle::Type::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            foreign_fn.result.clone().try_into()?,
        );

        let sym: Box<Symbol> = Box::new(Symbol {
            cif,
            ptr,
            parameter_types: foreign_fn.parameters,
            result_type: foreign_fn.result,
        });

        resource.symbols.insert(symbol_key, sym.clone());
    }

    // create new provider pointer

    let provider_pointer = resource.call_new(Rc::into_raw(Rc::new(props)) as *mut c_void)?;

    let registered_provider = RegisteredProvider {
        dylib: resource,
        ptr: provider_pointer,
    };

    providers.insert(provider_name, registered_provider);

    Ok(())
}

#[deno_core::op]
pub(crate) fn op_get_env(key: String) -> Result<Option<String>> {
    if key.is_empty() {
        return Err(type_error("Key is an empty string."));
    }

    if key.contains(&['=', '\0'] as &[char]) {
        return Err(type_error(format!(
            "Key contains invalid characters: {key:?}"
        )));
    }

    let r = match env::var(key) {
        Err(env::VarError::NotPresent) => None,
        v => Some(v?),
    };

    Ok(r)
}

#[deno_core::op]
pub fn as__client_print(msg: &str, is_err: bool) -> Result<()> {
    if is_err {
        js_log!(error, "{}", msg);
    } else {
        js_log!(warn, "{}", msg);
    }
    Ok(())
}

pub(crate) fn op_decls() -> Vec<::deno_core::OpDecl> {
    vec![
        op_get_env::decl(),
        as__client_print::decl(),
        as__client_new::decl(),
        as__runtime__register_provider__download::decl(),
        as__runtime__register_provider__allocate::decl(),
        as__runtime__resource_execute::decl(),
    ]
}
