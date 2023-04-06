use crate::Result;
use urn::{Urn, UrnBuilder};

pub trait Resource {
    const URN: Urn;
    const VERSION: u8;
    const NAME: &'static str;

    fn urn(&self) -> Result<Urn> {
        UrnBuilder::new("mashin", &format!("providers:{}", Self::NAME))
            .build()
            .map_err(Into::into)
    }

    fn version(&self) -> u8 {
        Self::VERSION
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }
}
