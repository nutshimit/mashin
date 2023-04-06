use deno_core::{
    error::{type_error, AnyError},
    resolve_path, serde_json, OpState, Resource, ResourceId,
};
use mashin_ffi::{DynamicLibraryResource, NativeType, NativeValue};
use libffi::middle::Arg;
use mashin_core::{
    sdk::{
        ext::anyhow::{anyhow, bail},
        ResourceAction, Result, Urn,
    },
    Client, ProviderInner, ProviderList, RawState,
};
use std::{
    alloc::{dealloc, Layout},
    cell::RefCell,
    collections::BTreeSet,
    env::{self, current_dir},
    ffi::c_void,
    ptr,
    rc::Rc,
    str::FromStr,
};

use super::state::{BackendState, FileState};

// `Atmosphere` `try_default`
#[deno_core::op]
pub(crate) async fn as__client_new(
    op_state: Rc<RefCell<OpState>>,
    backend_rid: Option<ResourceId>,
) -> Result<ResourceId> {
    let mut op_state = op_state.borrow_mut();

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

    let rid = op_state
        .resource_table
        .add(AtmosphereClient::new(backend_pointer, b"mysuperpassword").await?);

    op_state.put(AtmosphereState::default());
    Ok(rid)
}

#[deno_core::op]
pub(crate) async fn as__client_finished(
    op_state: Rc<RefCell<OpState>>,
    client_rid: ResourceId,
) -> Result<()> {
    let mut op_state = op_state.borrow_mut();
    let atmos_client = op_state
        .resource_table
        .get::<AtmosphereClient>(client_rid)?;

    let atmos_state = op_state.borrow_mut::<AtmosphereState>();

    let backend = unsafe {
        let handler = atmos_client.0.state_handler as *mut BackendState;
        &*handler
    };

    let resources_in_storage = backend.resources().await?;

    for diff in resources_in_storage.difference(&atmos_state.executed_resource) {
        println!("missing: {}, probably need to delete?", diff);
    }

    Ok(())
}

#[deno_core::op]
pub(crate) async fn as__runtime__provider__execute(
    rc_op_state: Rc<RefCell<OpState>>,
    client_rid: ResourceId,
    provider_rid: ResourceId,
    urn_str: String,
    config_raw: serde_json::Value,
    dry_run: bool,
) -> Result<serde_json::Value> {
    // urn:mashin:aws:s3:bucket/?=mysuper_bucket
    let urn = Urn::from_str(&urn_str)?;

    // op state to read the resource_table and in read only mode
    let rc_op_state_read_only = rc_op_state.clone();
    let mut op_state = rc_op_state_read_only.borrow_mut();
    let atmos_client = op_state
        .resource_table
        .get::<AtmosphereClient>(client_rid)?;

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

    // mashin state as mutable
    let atmos_state = op_state.borrow_mut::<AtmosphereState>();

    // backend engine (to save and get encrypted state)
    let backend = unsafe {
        let handler = atmos_client.0.state_handler as *mut BackendState;
        &*handler
    };

    let provider_in_state = atmos_state
        .providers
        .get(&provider_rid)
        .ok_or(anyhow!("invalid provider"))?;

    let config_ref = Box::new(config_raw);
    let config_pointer = Box::into_raw(config_ref) as *mut c_void;
    let urn_ref = Box::new(urn.clone());
    let urn_pointer = Box::into_raw(urn_ref) as *mut c_void;
    let action_ref = Box::new(if dry_run {
        ResourceAction::Get
    } else {
        ResourceAction::Create
    });
    let action_pointer = Box::into_raw(action_ref) as *mut c_void;

    let current_resource_state = backend
        .get(&urn)
        .await?
        .map(|s| s.decrypt(&atmos_client.0.key))
        .map_or(Ok(None), |v| v.map(Some))?
        .unwrap_or_default();

    let current_resource_state_ref = Box::new(current_resource_state);
    let current_resource_state_pointer = Box::into_raw(current_resource_state_ref) as *mut c_void;

    let state: RawState = unsafe {
        let pointer_type = NativeType::Pointer;
        let call_args = vec![
            NativeValue {
                pointer: provider_in_state.provider,
            }
            .as_arg(&pointer_type),
            NativeValue {
                pointer: urn_pointer,
            }
            .as_arg(&pointer_type),
            NativeValue {
                pointer: current_resource_state_pointer,
            }
            .as_arg(&pointer_type),
            NativeValue {
                pointer: config_pointer,
            }
            .as_arg(&pointer_type),
            NativeValue {
                pointer: action_pointer,
            }
            .as_arg(&pointer_type),
        ];

        // run(handle: *mut AwsProvider, urn: &Urn, config_ptr: *mut Value, dry_run_ptr: *mut bool,) -> Value
        let provider_state = run_symbol
            .cif
            .call::<*mut serde_json::Value>(run_symbol.ptr, &call_args);
        let state = &mut *provider_state;
        let clone_state = state.clone();

        // clean all pointers

        // drop the ffi state
        ptr::drop_in_place(provider_state);
        dealloc(
            provider_state as *mut u8,
            Layout::new::<serde_json::Value>(),
        );

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

        clone_state.into()
    };

    let encrypted = state.encrypt(&atmos_client.0.key)?;
    backend.save(&urn, &encrypted).await?;

    // TODO write diff output to a state for each resource

    atmos_state.executed_resource.insert(urn);

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
    let client = op_state.borrow_mut::<AtmosphereState>();

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

pub(crate) fn op_decls() -> Vec<::deno_core::OpDecl> {
    vec![
        op_get_env::decl(),
        as__client_new::decl(),
        as__client_finished::decl(),
        as__runtime__register_provider__allocate::decl(),
        as__runtime__provider__execute::decl(),
    ]
}

// Re export client  to be able
// to impl Resource on it
pub(crate) struct AtmosphereClient(pub Client);

impl AtmosphereClient {
    pub async fn new(backend_pointer: *mut c_void, passphrase: &[u8]) -> Result<Self> {
        Ok(Self(Client::new(backend_pointer, passphrase).await?))
    }
}

impl Resource for AtmosphereClient {} // Blank impl

#[derive(Default)]
pub struct AtmosphereState {
    providers: ProviderList,
    executed_resource: BTreeSet<Urn>,
    rid: ResourceId,
}

// drop all providers alloc
impl Drop for AtmosphereState {
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
