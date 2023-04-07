use crate::{js_log, log};
use deno_core::{
    error::{type_error, AnyError},
    resolve_path, serde_json, OpState, Resource, ResourceId,
};
use libffi::middle::Arg;
use mashin_core::{
    sdk::{
        ext::anyhow::{anyhow, bail},
        ResourceAction, ResourceDiff, ResourceResult, Result, Urn,
    },
    Client, ProviderInner, ProviderList, RawState,
};
use mashin_ffi::{DynamicLibraryResource, NativeType, NativeValue};
use std::{
    alloc::{dealloc, Layout},
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap},
    env::{self, current_dir},
    ffi::c_void,
    ptr,
    rc::Rc,
    str::FromStr,
};

use super::state::{BackendState, FileState};

// `Mashin` `try_default`
#[deno_core::op]
pub(crate) async fn as__client_new(
    op_state: Rc<RefCell<OpState>>,
    backend_rid: Option<ResourceId>,
) -> Result<ResourceId> {
    log!(info, "Engine is starting up...");

    let mut op_state = op_state.borrow_mut();

    log!(info, "Getting state lock...");
    let backend = Box::new({
        if let Some(backend_rid) = backend_rid {
            // should be handled by ffi
            todo!()
        } else {
            let path = resolve_path(".mashin", current_dir()?.as_path())?
                .to_file_path()
                .or_else(|_| bail!("unable to resolve state dir"))?;
            // create new file   state
            BackendState::Local(FileState::new(path)?)
        }
    });
    let backend_pointer = Box::into_raw(backend) as *mut c_void;

    log!(info, "Engine launched successfully");

    let rid = op_state
        .resource_table
        .add(MashinClient::new(backend_pointer, b"mysuperpassword").await?);

    op_state.put(MashinState::default());
    Ok(rid)
}

#[deno_core::op]
pub(crate) async fn as__client_apply(
    op_state: Rc<RefCell<OpState>>,
    client_rid: ResourceId,
    providers_map: Vec<(String, ResourceId)>,
) -> Result<()> {
    let mut op_state = op_state.borrow_mut();
    let mashin_client = op_state.resource_table.get::<MashinClient>(client_rid)?;

    let backend = unsafe {
        let handler = mashin_client.0.state_handler as *mut BackendState;
        &*handler
    };

    // prepare all resource
    let mut providers_by_name = HashMap::new();
    for (provider_name, provider_rid) in providers_map {
        let symbols = {
            let resource = op_state
                .resource_table
                .get::<DynamicLibraryResource>(provider_rid)?;
            resource
                .symbols
                .get("run")
                .ok_or(anyhow!("invalid function"))?
                .clone()
        };
        providers_by_name.insert(provider_name, symbols);
    }

    let resources_in_storage = backend.resources().await?;
    let mashin_state = op_state.borrow_mut::<MashinState>();

    // delete all resource that are missing
    let mut to_delete = Vec::new();
    for res in resources_in_storage {
        if !mashin_state.executed_resource.contains_key(&res) {
            to_delete.push(res);
        }
    }

    // all executed resource previously
    let mut to_update = Vec::new();
    let mut to_create = Vec::new();
    for (urn, diff) in &mashin_state.executed_resource {
        // already exist
        if mashin_state.executed_resource.contains_key(&urn) {
            to_update.push((urn, diff));
        }
        // should create
        else {
            to_create.push((urn, diff));
        }
    }

    log!(warn, "You have {} resources to delete", to_delete.len());
    log!(info, "You have {} resources to create", to_create.len());
    log!(info, "You have {} resources to update", to_update.len());

    if dialoguer::Confirm::new()
        .with_prompt("\nAre you sure you want to continue?")
        .interact()?
    {
        for urn in to_delete {
            let parsed_urn = ParsedResourceUrn::try_from(urn)?;
            let delete_fn = providers_by_name
                .get(&parsed_urn.provider)
                .ok_or(anyhow!("invalid function"))?;
        }
        log!(info, "Updating the resouroces...");
    } else {
        log!(
            info,
            "Your job has been cancelled! Nothing has been saved to the state."
        );
    }

    Ok(())
}

// run in dry run mode and register the current state then we can compare
// in `as__client_apply` for all resources and then apply only the changes
// we want
#[deno_core::op]
pub(crate) async fn as__runtime__provider__dry_run(
    rc_op_state: Rc<RefCell<OpState>>,
    client_rid: ResourceId,
    provider_rid: ResourceId,
    urn_str: String,
    config_raw: serde_json::Value,
) -> Result<serde_json::Value> {
    log!(info, "Refreshing {}...", urn_str);

    // the URN of the resource
    // urn:mashin:aws:s3:bucket/?=mysuper_bucket
    // q_component is the resource name
    let urn = Urn::from_str(&urn_str)?;

    // op state to read the resource_table and in read only mode
    let rc_op_state_read_only = rc_op_state.clone();
    let mut op_state = rc_op_state_read_only.borrow_mut();
    let mashin_client = op_state.resource_table.get::<MashinClient>(client_rid)?;

    // grab the run symbol for this resource before we borrow the mashin state as mutable
    let run_symbol = {
        let resource = op_state
            .resource_table
            .get::<DynamicLibraryResource>(provider_rid)?;
        let symbols = &resource.symbols;
        *symbols
            .get("run")
            .ok_or_else(|| type_error("Invalid FFI symbol name"))?
            .clone()
    };

    // get the current mashin state as mutable
    let mashin_state = op_state.borrow_mut::<MashinState>();

    // backend engine (to save and get current encrypted state for this resource)
    let backend = unsafe {
        let handler = mashin_client.0.state_handler as *mut BackendState;
        &*handler
    };

    // get the current provider registered in `as__runtime__register_provider__allocate`
    // we can get the FFI pointer from the mashin_ffi ext
    let provider_in_state = mashin_state
        .get_provider_by_rid(&provider_rid)
        .ok_or(anyhow!("invalid provider"))?;

    let current_resource_state = Rc::new(
        backend
            .get(&urn)
            .await?
            .map(|s| s.decrypt(&mashin_client.0.key))
            .map_or(Ok(None), |v| v.map(Some))?
            .unwrap_or_default(),
    );

    // setup pointers, see the `pointers_cleanup` that get run when the dry run is finished
    let current_resource_state_pointer =
        Rc::into_raw(current_resource_state.clone()) as *mut c_void;
    let config_pointer = Box::into_raw(Box::new(config_raw)) as *mut c_void;
    let urn_pointer = Box::into_raw(Box::new(urn.clone())) as *mut c_void;
    let action_pointer = Box::into_raw(Box::new(ResourceAction::Get)) as *mut c_void;
    let empty_diff_pointer = Box::into_raw(Box::new(ResourceDiff::default())) as *mut c_void;

    // if you need to add a pointer above, make sure it get added in the
    // cleanup function below as well
    let pointers_cleanup = || unsafe {
        // drop the config
        ptr::drop_in_place(config_pointer);
        dealloc(
            config_pointer as *mut u8,
            Layout::new::<serde_json::Value>(),
        );

        // drop the urn
        ptr::drop_in_place(urn_pointer);
        dealloc(urn_pointer as *mut u8, Layout::new::<Urn>());

        // drop the action
        ptr::drop_in_place(action_pointer);
        dealloc(action_pointer as *mut u8, Layout::new::<ResourceAction>());

        // drop the empty_diff
        ptr::drop_in_place(empty_diff_pointer);
        dealloc(empty_diff_pointer as *mut u8, Layout::new::<ResourceDiff>());
    };

    let state: RawState = unsafe {
        let call_args = vec![
            NativeValue {
                pointer: provider_in_state.provider,
            }
            .as_arg(&NativeType::Pointer),
            NativeValue {
                pointer: urn_pointer,
            }
            .as_arg(&NativeType::Pointer),
            NativeValue {
                pointer: current_resource_state_pointer,
            }
            .as_arg(&NativeType::Pointer),
            NativeValue {
                pointer: config_pointer,
            }
            .as_arg(&NativeType::Pointer),
            NativeValue {
                pointer: action_pointer,
            }
            .as_arg(&NativeType::Pointer),
            // diff is only used on update
            NativeValue {
                pointer: empty_diff_pointer,
            }
            .as_arg(&NativeType::Pointer),
        ];

        // execute the ffi run() function for this provider with arguments
        // defined above
        let provider_state = run_symbol
            .cif
            .call::<*mut ResourceResult>(run_symbol.ptr, &call_args);
        let state = &mut *provider_state;
        let clone_state = state.clone();

        // drop the provider state and use the cloned state above instead
        ptr::drop_in_place(provider_state);
        dealloc(provider_state as *mut u8, Layout::new::<ResourceResult>());

        clone_state.inner().into()
    };

    // Insert the diff
    let diff = state.compare_with(&current_resource_state, None, false);
    mashin_state.executed_resource.insert(urn, diff);

    pointers_cleanup();

    Ok(state.generate_ts_output())
}

#[deno_core::op]
pub(crate) async fn as__runtime__register_provider__allocate(
    rc_op_state: Rc<RefCell<OpState>>,
    provider_rid: ResourceId, // should be registered with ffi first
    provider_name: String,
) -> Result<()> {
    let get_symbol = |fn_name: &str| {
        let op_state = rc_op_state.borrow_mut();
        let resource = op_state
            .resource_table
            .get::<DynamicLibraryResource>(provider_rid)
            .expect("valid dylib");
        let symbols = &resource.symbols;
        *symbols.get(fn_name).expect("valid symbol").clone()
    };
    // symbol of the `drop` function in the provider ffi
    let drop_symbol = get_symbol("drop");
    // symbol of the `new` function in the provider ffi
    let symbol = get_symbol("new");

    // create the new provider (calling new)
    let call_args: Vec<Arg> = Vec::new();
    let provider_pointer = unsafe { symbol.cif.call::<*mut c_void>(symbol.ptr, &call_args) };

    let mut op_state = rc_op_state.borrow_mut();
    let client = op_state.borrow_mut::<MashinState>();

    if client.providers.get(&provider_rid).is_none() {
        client.providers.insert(
            provider_rid,
            ProviderInner {
                name: provider_name,
                provider: provider_pointer,
                drop_fn: drop_symbol,
            },
        );
    }

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
        as__client_apply::decl(),
        as__runtime__register_provider__allocate::decl(),
        as__runtime__provider__dry_run::decl(),
    ]
}

// Re export client  to be able
// to impl Resource on it
pub(crate) struct MashinClient(pub Client);

impl MashinClient {
    pub async fn new(backend_pointer: *mut c_void, passphrase: &[u8]) -> Result<Self> {
        Ok(Self(Client::new(backend_pointer, passphrase).await?))
    }
}

impl Resource for MashinClient {} // Blank impl

#[derive(Default)]
pub struct MashinState {
    providers: ProviderList,
    executed_resource: BTreeMap<Urn, ResourceDiff>,
    rid: ResourceId,
}

impl MashinState {
    pub fn get_provider_by_rid(&self, rid: &ResourceId) -> Option<ProviderInner> {
        self.providers.get(rid).cloned()
    }
}

// drop all providers alloc
impl Drop for MashinState {
    fn drop(&mut self) {
        let drop_provider = |(_, inner): (_, &ProviderInner)| {
            unsafe {
                let call_args = vec![NativeValue {
                    pointer: inner.provider,
                }
                .as_arg(inner.drop_fn.parameter_types.get(0).unwrap())];
                inner.drop_fn.cif.call::<()>(inner.drop_fn.ptr, &call_args);
            };
        };
        self.providers.iter().for_each(drop_provider)
    }
}

pub struct ParsedResourceUrn {
    provider: String,
    urn: Urn,
}

impl TryFrom<Urn> for ParsedResourceUrn {
    type Error = AnyError;

    fn try_from(urn: Urn) -> std::result::Result<Self, Self::Error> {
        let nss: Vec<&str> = urn.nss().split(":").collect();
        let provider = nss
            .first()
            .ok_or(anyhow!("invalid provider"))
            .cloned()?
            .to_string();

        Ok(Self { provider, urn })
    }
}
