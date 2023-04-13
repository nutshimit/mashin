use aws_sdk_s3::model::BucketCannedAcl;
use mashin_sdk::{
    ext::{
        anyhow::{anyhow, bail},
        async_trait::async_trait,
        serde::{ser::SerializeStruct, Deserialize, Serialize},
        serde_json::{self, Value},
        tokio,
    },
    resource, CliLogger, Provider, ProviderState, Resource, ResourceAction, ResourceArgs,
    ResourceDiff, ResourceResult, Result, Urn,
};

use std::{
    alloc::{dealloc, Layout},
    cell::RefCell,
    env,
    str::FromStr,
};
use std::{any::Any, ptr};
use std::{rc::Rc, sync::Arc};

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct AwsConfig {
    aws_key: Option<String>,
}

#[derive(Default, Clone)]
pub struct AwsState {
    aws_client: Option<Arc<aws_sdk_s3::Client>>,
}

#[derive(Default)]
pub struct AwsProvider {
    __config: AwsConfig,
    __state: Rc<RefCell<ProviderState>>,
}

#[async_trait]
impl Provider for AwsProvider {
    async fn init(&mut self) -> Result<()> {
        let config = aws_config::load_from_env().await;
        let client = aws_sdk_s3::Client::new(&config);

        let default_state = AwsState {
            aws_client: Some(Arc::new(client)),
        };
        self.__state.put(default_state);

        Ok(())
    }

    fn state(&self) -> &ProviderState {
        self.__state.as_ref()
    }

    // this can be generated by the macro
    fn __from_current_state(
        &self,
        urn: &Rc<Urn>,
        state: &Rc<RefCell<Value>>,
    ) -> Result<Rc<RefCell<dyn Resource>>> {
        let raw_urn = urn.nss().split(":").collect::<Vec<_>>()[1..].join(":");
        // expect; s3:bucket
        let module_urn = raw_urn.to_lowercase();
        // resource name
        let name = urn
            .q_component()
            .ok_or(anyhow!("expect valid urn (name not found)"))?;

        // map module dynamicly
        match module_urn.as_str() {
            "s3:bucket" => Ok(Bucket::from_current_state(
                name,
                &urn.to_string(),
                state.clone(),
            )?),
            _ => bail!("invalid URN"),
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BucketConfig {
    #[serde(default)]
    acl: Option<String>,
    #[serde(default)]
    woot: Option<bool>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeepConfig {
    #[serde(default)]
    deep1: DeepConfig2,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeepConfig2 {
    #[serde(default)]
    deep2: DeepConfig3,
    #[serde(default)]
    test: Option<String>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeepConfig3 {
    #[serde(default)]
    deep3: bool,
}

#[resource]
pub struct Bucket {
    url: Option<String>,
    #[sensitive]
    password: Option<String>,
    deep: DeepConfig,
    test: Option<String>,
    // this should be injected by the macro
    __name: String,
    __config: BucketConfig,
    __urn: String,
}

#[async_trait]
impl Resource for Bucket {
    async fn get(&mut self, provider_state: &ProviderState) -> Result<()> {
        log!(trace, "Refreshing ");

        let state = provider_state
            .try_borrow::<AwsState>()
            .ok_or(anyhow!("state not initialized"))?;
        let client = state.aws_client.as_ref().ok_or(anyhow!("invalid state"))?;

        let bucket_exist = client
            .head_bucket()
            .bucket(&self.__name)
            .send()
            .await
            .is_ok();

        if bucket_exist {
            self.url = Some(format!("http://{}.s3.amazonaws.com/", &self.__name));
        }

        Ok(())
    }

    async fn update(&mut self, provider_state: &ProviderState, diff: &ResourceDiff) -> Result<()> {
        // we should update ACL
        if diff.has_change("config.acl") {
            log!(info, "SHOULD UPDATE ACL");
        }

        Ok(())
    }

    async fn delete(&mut self, provider_state: &ProviderState) -> Result<()> {
        log!(info, "SHOULD DELETE");
        Ok(())
    }

    async fn create(&mut self, provider_state: &ProviderState) -> Result<()> {
        let state = provider_state
            .try_borrow::<AwsState>()
            .ok_or(anyhow!("state not initialized"))?;

        let client = state.aws_client.as_ref().ok_or(anyhow!("invalid user"))?;

        let cfg = aws_sdk_s3::model::CreateBucketConfiguration::builder()
            .location_constraint(aws_sdk_s3::model::BucketLocationConstraint::UsEast2)
            .build();

        let acl = self
            .__config
            .acl
            .as_ref()
            .map(|a| BucketCannedAcl::from_str(a).expect("valid acl"))
            .unwrap_or(BucketCannedAcl::Private);

        let bucket = client
            .create_bucket()
            .create_bucket_configuration(cfg)
            .acl(acl)
            .bucket(&self.__name)
            .send()
            .await?;

        self.url = bucket.location().map(|s| s.to_string());

        Ok(())
    }

    // these fns can be generated with the macro
    fn name(&self) -> &str {
        self.__name.as_str()
    }

    fn __default_with_params(name: &str, urn: &str) -> Self
    where
        Self: Sized,
    {
        Self {
            __name: name.to_string(),
            __urn: urn.to_string(),
            ..Default::default()
        }
    }

    fn __set_config_from_value(&mut self, config: &Rc<Value>) {
        let config = config.as_ref().clone();
        self.__config = serde_json::from_value::<BucketConfig>(config).unwrap_or_default();
    }
}

#[no_mangle]
pub extern "C" fn new(logger_ptr: *mut &'static CliLogger) -> *mut AwsProvider {
    __MASHIN_LOG_INIT.call_once(|| {
        let logger = unsafe {
            let logger = Box::from_raw(logger_ptr);
            logger
        };

        // FIXME: pass it from the cli level
        log::set_max_level(log::LevelFilter::Info);
        log::set_boxed_logger(Box::new(logger)).expect("valid logger");
        setup_panic_hook();
    });

    let runtime = tokio::runtime::Runtime::new().expect("New runtime");
    let mut provider = AwsProvider::default();
    runtime.block_on(provider.init()).expect("valid provider");
    let static_ref = Box::new(provider);
    Box::into_raw(static_ref)
}

#[no_mangle]
pub extern "C" fn run(
    handle_ptr: *mut AwsProvider,
    args_ptr: *mut ResourceArgs,
) -> *mut ResourceResult {
    let runtime = tokio::runtime::Runtime::new().expect("New runtime");

    // grab current provider
    assert!(!handle_ptr.is_null());
    let provider = unsafe {
        let provider = &mut *handle_ptr;
        provider
    };
    let provider_state = provider.state();

    // resource URN
    let args = unsafe { Rc::from_raw(args_ptr) };

    let urn = &args.urn;
    let raw_config = &args.raw_config.clone();
    let raw_state = &args.raw_state.clone();

    let resource = provider
        .__from_current_state(urn, raw_state)
        .expect("Valid resource");

    let mut resource = resource.borrow_mut();

    // grab the state before applying our values
    resource.__set_config_from_value(raw_config);

    runtime
        .block_on(async {
            match args.action.as_ref() {
                ResourceAction::Update { diff } => resource.update(provider_state, diff),
                ResourceAction::Create => resource.create(provider_state),
                ResourceAction::Delete => resource.delete(provider_state),
                ResourceAction::Get => resource.get(provider_state),
            }
            .await
        })
        .expect("valid execution");

    let state = resource.to_raw_state().expect("valid resource");
    let result = ResourceResult::new(state);

    Rc::into_raw(Rc::new(result)) as *mut ResourceResult
}

#[no_mangle]
pub extern "C" fn drop(handle: *mut AwsProvider) {
    assert!(!handle.is_null());
    unsafe {
        ptr::drop_in_place(handle);
        dealloc(handle as *mut u8, Layout::new::<AwsProvider>());
    }
}

impl Drop for AwsProvider {
    fn drop(&mut self) {
        log!(trace, "AWS PROVIDER DROPPED")
    }
}

fn setup_panic_hook() {
    let orig_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        eprintln!("\n============================================================");
        eprintln!("Mashin has panicked. This is a bug in Mashin. Please report this");
        eprintln!("at https://github.com/nutshimit/mashin/issues/new.");
        eprintln!("If you can reliably reproduce this panic, include the");
        eprintln!("reproduction steps and re-run with the RUST_BACKTRACE=1 env");
        eprintln!("var set and include the backtrace in your report.");
        eprintln!();
        eprintln!("Platform: {} {}", env::consts::OS, env::consts::ARCH);
        eprintln!("Version: {}", env!("CARGO_PKG_VERSION"));
        eprintln!("Args: {:?}", env::args().collect::<Vec<_>>());
        eprintln!();
        orig_hook(panic_info);
        std::process::exit(1);
    }));
}
