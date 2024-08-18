//! Services provide access to media storage.

use std::str::FromStr;

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
pub struct Service {
    pub(crate) service: String,
    pub(crate) config: std::collections::HashMap<String, String>,
}

impl Service {
    pub fn new(service: String, config: std::collections::HashMap<String, String>) -> Self {
        Service { service, config }
    }
}

impl Default for Service {
    fn default() -> Self {
        let working_dir = std::env::current_dir()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        let mut map = std::collections::HashMap::new();
        map.insert("root".to_string(), working_dir);

        Service {
            service: "fs".to_string(),
            config: map,
        }
    }
}

impl Service {
    pub(crate) fn operator(&self) -> Result<opendal::Operator, crate::Error> {
        let op = {
            let scheme = match opendal::Scheme::from_str(&self.service) {
                Ok(scheme) => scheme,
                Err(_) => {
                    return Err(crate::Error::AVError(format!(
                        "unsupported service `{}`",
                        self.service
                    )));
                }
            };
            let map = self.config.clone();
            match opendal::Operator::via_map(scheme, map) {
                Ok(op) => op,
                Err(_) => {
                    return Err(crate::Error::AVError(format!(
                        "failed to instantiate service {} with config {:?}",
                        self.service, self.config
                    )));
                }
            }
        };

        Ok(op)
    }

    pub(crate) fn blocking_operator(
        &self,
        io_runtime: &tokio::runtime::Handle,
    ) -> Result<opendal::BlockingOperator, crate::Error> {
        let op = self.operator()?;

        let bop = if op.info().full_capability().blocking {
            op.blocking()
        } else {
            let _x = io_runtime.enter();
            op.layer(opendal::layers::BlockingLayer::create().unwrap())
                .blocking()
        };

        Ok(bop)
    }
}
