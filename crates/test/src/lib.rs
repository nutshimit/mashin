use aws_sdk_s3::model::BucketCannedAcl;
use mashin_sdk::{
    compare_json_objects_recursive,
    ext::{
        anyhow::anyhow,
        async_trait::async_trait,
        serde::{ser::SerializeStruct, Deserialize, Serialize},
        serde_json::{from_value, json, to_value, Value},
        tokio,
    },
    merge_json, resource, Provider, ProviderState, Resource, ResourceAction, Result, Urn,
};

use std::ptr;
use std::sync::Arc;
use std::{
    alloc::{dealloc, Layout},
    str::FromStr,
};

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
    __state: Box<ProviderState>,
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
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BucketConfig {
    #[serde(default)]
    acl: Option<String>,
    #[serde(default)]
    woot: Option<bool>,
}

#[resource]
pub struct Bucket {
    url: Option<String>,

    // this should be injected by the macro?
    #[sensitive]
    __name: String,
    #[sensitive]
    __config: BucketConfig,
    #[sensitive]
    __urn: String,
}

#[async_trait]
impl Resource for Bucket {
    async fn get(&mut self, provider_state: &ProviderState) -> Result<bool> {
        let state = provider_state
            .try_borrow::<AwsState>()
            .ok_or(anyhow!("state not initialized"))?;
        let client = state.aws_client.as_ref().ok_or(anyhow!("invalid user"))?;

        let bucket_exist = client
            .head_bucket()
            .bucket(&self.__name)
            .send()
            .await
            .is_ok();

        if bucket_exist {
            self.url = Some(format!("http://{}.s3.amazonaws.com/", &self.__name));
        }

        Ok(bucket_exist)
    }

    async fn update(&mut self, provider_state: &ProviderState) -> Result<()> {
        println!("SHOULD UPDATE");
        Ok(())
    }

    async fn delete(&mut self, provider_state: &ProviderState) -> Result<()> {
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
}

#[no_mangle]
pub extern "C" fn new() -> *mut AwsProvider {
    let runtime = tokio::runtime::Runtime::new().expect("New runtime");
    let mut provider = AwsProvider::default();
    runtime.block_on(provider.init()).expect("valid provider");
    let static_ref = Box::new(provider);
    Box::into_raw(static_ref)
}

#[no_mangle]
pub extern "C" fn run(
    handle_ptr: *mut AwsProvider,
    urn_ptr: *mut Urn,
    resource_state_ptr: *mut Value,
    config_ptr: *mut Value,
    action_ptr: *mut ResourceAction,
) -> *mut Value {
    assert!(!handle_ptr.is_null());
    let state = unsafe {
        let provider = &mut *handle_ptr;
        provider.state()
    };

    let urn = unsafe { &*urn_ptr };
    let config = unsafe { &*config_ptr };

    // FIXME: should be dropped as we drop the ref
    let mut resource_state = unsafe { &*resource_state_ptr }.clone();
    let is_empty_state = resource_state.as_null().is_some();
    let action = unsafe { &*action_ptr };

    // if the state isnt empty
    if !is_empty_state {
        let merge_fields = json!({
            "__name": {
                "value": urn.q_component().expect("valid resource name").to_string(),
                "sensitive": true,
            },
            "__urn": {
                "value": urn.to_string(),
                "sensitive": true,
            },
        });

        merge_json(&mut resource_state, &merge_fields);
    }

    let runtime = tokio::runtime::Runtime::new().expect("New runtime");

    // urn: urn:provider:aws:s3:bucket?=mysuper_bucket
    // expect: s3:bucket
    let res_state = match urn.nss().split(":").collect::<Vec<_>>()[1..] {
        ["s3", "bucket"] => {
            let mut resource = if !is_empty_state {
                from_value::<Bucket>(resource_state.clone()).expect("valid state")
            } else {
                Bucket {
                    __name: urn.q_component().expect("valid resource name").to_string(),
                    __urn: urn.to_string(),
                    ..Default::default()
                }
            };

            runtime.block_on(async {
                match action {
                    ResourceAction::Update => {
                        resource.__config =
                            from_value::<BucketConfig>(config.clone())
                                .expect("valid config");
                        resource.update(state).await.expect("valid resource");
                    }
                    ResourceAction::Delete => {
                        resource.__config =
                            from_value::<BucketConfig>(config.clone())
                                .expect("valid config");
                        resource.delete(state).await.expect("valid resource");
                    }
                    ResourceAction::Create => {
                        let current_state = resource.clone();

                        resource.__config =
                            from_value::<BucketConfig>(config.clone())
                                .expect("valid config");

                        let resource_exist = resource.get(state).await.expect("valid resource");

                        if !resource_exist {

                            let merge_fields = json!({
                                "__name": {
                                    "value": urn.q_component().expect("valid resource name").to_string(),
                                    "sensitive": true,
                                },
                                "__urn": {
                                    "value": urn.to_string(),
                                    "sensitive": true,
                                },
                            });

                            merge_json(&mut resource_state, &merge_fields);

                            resource.create(state).await.expect("valid resource");
                        } else if current_state != resource {
                            let current_state_json =
                                to_value(current_state)
                                    .expect("valid resource");
                            let resource_json = to_value(resource.clone())
                                .expect("valid resource");

                            let changed = compare_json_objects_recursive(
                                &current_state_json,
                                &resource_json,
                                None,
                                false,
                            );

                            println!("CHANGED FIELD {:?}", changed);
                            resource.update(state).await.expect("valid resource");
                        }
                    }
                    ResourceAction::Get => {
                        resource.__config =
                            from_value::<BucketConfig>(config.clone())
                                .expect("valid config");
                        resource.get(state).await.expect("valid resource");
                    }
                }
            });

            to_value(resource).expect("valid resource")
        }
        _ => panic!("invalid nss {}", urn.nss()),
    };
    let static_ref = Box::new(res_state);
    Box::into_raw(static_ref)
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
        println!("AWS PROVIDER DROPPED")
    }
}
