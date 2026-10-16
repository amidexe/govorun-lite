use govorun_core::{self_hosted_flow::configuration::Configuration, Future};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Configuration,
    Network,
    Http(u16),
    InvalidResponse,
}

pub trait Network: Send + Sync {
    fn complete<'a>(
        &'a self,
        configuration: &'a Configuration,
        key: Option<&'a str>,
        text: &'a str,
    ) -> Future<'a, Result<String, Failure>>;
}
