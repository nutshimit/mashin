#[mashin_sdk::provider(name = "aws")]
mod provider {

    use mashin_sdk::{ProviderBuilder, ProviderDefault, ResourceDefault, Result};
    use std::sync::Arc;

    #[mashin::provider]
    pub struct Provider;

    #[mashin::config]
    pub struct Config {
        aws_key: Option<String>,
    }

    #[mashin::state]
    pub struct State {
        aws_client: Option<Arc<aws_sdk_s3::Client>>,
    }

    #[mashin::builder]
    impl ProviderBuilder for Provider {
        async fn build(&mut self) -> mashin_sdk::Result<()> {
            let config = aws_config::load_from_env().await;
            let client = aws_sdk_s3::Client::new(&config);

            let default_state = State {
                aws_client: Some(Arc::new(client)),
            };

            self.state().put(default_state);

            Ok(())
        }
    }

    impl Drop for Provider {
        fn drop(&mut self) {
            log!(info, "AWS PROVIDER DROPPED")
        }
    }

    #[mashin::resource_config]
    pub struct BucketConfig {
        acl: Option<String>,
        woot: Option<bool>,
    }

    #[mashin::resource(name = "s3:bucket", config = BucketConfig)]
    pub struct Bucket {
        url: Option<String>,
        #[sensitive]
        password: Option<String>,
        test: Option<String>,
    }

    #[mashin::calls]
    impl mashin_sdk::Resource for Bucket {
        async fn get(&mut self, provider_state: &mashin_sdk::ProviderState) -> Result<()> {
            log!(info, "Refreshing ");

            let state = provider_state
                .try_borrow::<State>()
                .ok_or(mashin_sdk::ext::anyhow::anyhow!("state not initialized"))?;

            let client = state
                .aws_client
                .as_ref()
                .ok_or(mashin_sdk::ext::anyhow::anyhow!("invalid state"))?;

            let bucket_exist = client
                .head_bucket()
                .bucket(self.name())
                .send()
                .await
                .is_ok();

            if bucket_exist {
                self.url = Some(format!("http://{}.s3.amazonaws.com/", self.name()));
            }

            Ok(())
        }
        async fn create(&mut self, provider_state: &mashin_sdk::ProviderState) -> Result<()> {
            todo!()
        }
        async fn delete(&mut self, provider_state: &mashin_sdk::ProviderState) -> Result<()> {
            todo!()
        }
        async fn update(
            &mut self,
            provider_state: &mashin_sdk::ProviderState,
            diff: &mashin_sdk::ResourceDiff,
        ) -> Result<()> {
            todo!()
        }
    }
}

mod test {
    fn test() {
        //let t = Bucket {};
    }
}
