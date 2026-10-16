//! Typed requests over the agent transport. The agent supplies capabilities,
//! storage and OS effects; it never becomes a second feature owner.
pub mod storage;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::{collections::BTreeSet, future::Future, pin::Pin, sync::Arc, time::Duration};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability(String);
impl Capability {
    pub fn named(method: &str) -> Result<Self, &'static str> {
        if crate::features::capabilities::supported(method) {
            Ok(Self(method.into()))
        } else {
            Err("unsupported_command")
        }
    }
    pub fn name(&self) -> &str {
        &self.0
    }
}

pub struct Request<P> {
    pub capability: Capability,
    pub params: P,
}

pub use govorun_core::features::cancellation::Cancellation;

pub type AgentFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, &'static str>> + Send + 'a>>;
pub type Diagnostics = Arc<dyn Fn() + Send + Sync>;
pub type Connectivity = Arc<dyn Fn(bool, u64, u64) + Send + Sync>;
pub type Generation = Arc<dyn Fn() -> Option<u64> + Send + Sync>;
pub trait AgentTransport: Send + Sync {
    fn bind_benchmark(
        &self,
        _: Arc<crate::features::benchmark_runner::Runner>,
    ) -> Result<(), &'static str> {
        Err("benchmark_unsupported")
    }
    fn bind_diagnostics(&self, _: Diagnostics) -> Result<(), &'static str> {
        Ok(())
    }
    fn bind_connectivity(&self, _: Connectivity) -> Result<(), &'static str> {
        Ok(())
    }
    fn bind_generation(&self, _: Generation) -> Result<(), &'static str> {
        Ok(())
    }
    fn invoke<'a>(&'a self, method: &'a str, params: Value) -> AgentFuture<'a>;
    fn self_host_secret<'a>(&'a self, method: &'a str, params: Value) -> AgentFuture<'a> {
        self.invoke(method, params)
    }
}
#[derive(Clone)]
pub struct AgentClient {
    transport: Arc<dyn AgentTransport>,
    capabilities: BTreeSet<String>,
}
impl AgentClient {
    pub fn from_snapshot<T: AgentTransport + 'static>(service: T, snapshot: &Value) -> Result<Self, &'static str> {
        Self::with_transport(Arc::new(service), snapshot)
    }
    pub fn with_transport(
        transport: Arc<dyn AgentTransport>,
        snapshot: &Value,
    ) -> Result<Self, &'static str> {
        let entries = snapshot
            .get("capabilities")
            .and_then(Value::as_array)
            .ok_or("native_invalid_capabilities")?;
        if entries.len() > crate::features::capabilities::agent_capability_count() {
            return Err("native_invalid_capabilities");
        }
        let mut capabilities = BTreeSet::new();
        for entry in entries {
            let name = entry.as_str().ok_or("native_invalid_capabilities")?;
            Capability::named(name).map_err(|_| "native_invalid_capabilities")?;
            if !capabilities.insert(name.to_owned()) {
                return Err("native_invalid_capabilities");
            }
        }
        Ok(Self {
            transport,
            capabilities,
        })
    }
    pub fn bind_benchmark(
        &self,
        owner: Arc<crate::features::benchmark_runner::Runner>,
    ) -> Result<(), &'static str> {
        self.transport.bind_benchmark(owner)
    }
    pub fn bind_diagnostics(&self, changed: Diagnostics) -> Result<(), &'static str> {
        self.transport.bind_diagnostics(changed)
    }
    pub fn bind_connectivity(&self, handler: Connectivity) -> Result<(), &'static str> {
        self.transport.bind_connectivity(handler)
    }
    pub fn bind_generation(&self, generation: Generation) -> Result<(), &'static str> {
        self.transport.bind_generation(generation)
    }
    pub fn supports(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability.name())
    }

    pub async fn self_host_secret<R: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> Result<R, &'static str> {
        if !matches!(method, "secret.read" | "secret.write" | "secret.delete")
            || !matches!(
                params["key"].as_str(),
                Some("selfHost.ollama" | "selfHost.openai" | "selfHost.gemini")
            )
        {
            return Err("unsupported_command");
        }
        let capability = Capability::named(method)?;
        if !self.supports(&capability) {
            return Err("native_capability_unavailable");
        }
        let reply = self.transport.self_host_secret(method, params).await?;
        serde_json::from_value(reply).map_err(|_| "native_invalid_response")
    }

    /// Shared physical-call wrapper; capability, budget and cancellation admission
    /// remain in the one typed call owner below.
    pub async fn call_named<P: Serialize, R: DeserializeOwned>(
        &self, method: &str, params: P, budget: Duration,
    ) -> Result<R, &'static str> {
        self.call(Request { capability: Capability::named(method)?, params },
            budget, &Cancellation::default()).await
    }

    pub async fn call<P: Serialize, R: DeserializeOwned>(
        &self,
        request: Request<P>,
        budget: Duration,
        cancellation: &Cancellation,
    ) -> Result<R, &'static str> {
        if cancellation.is_cancelled() {
            return Err("native_cancelled");
        }
        if !self.supports(&request.capability) {
            return Err("native_capability_unavailable");
        }
        if budget.is_zero() || budget > Duration::from_secs(20) {
            return Err("native_invalid_timeout");
        }
        let params = serde_json::to_value(request.params).map_err(|_| "invalid_command")?;
        if !params.is_object() {
            return Err("invalid_command");
        }
        // Cancellation stops waiting and rejects the result. It does not claim
        // that an already dispatched OS operation or durable write was undone.
        let reply = tokio::select! {
            biased;
            _ = cancellation.cancelled() => return Err("native_cancelled"),
            result = tokio::time::timeout(budget, self.transport.invoke(request.capability.name(), params)) =>
                result.map_err(|_| "native_timeout")??,
        };
        serde_json::from_value(reply).map_err(|_| "native_invalid_response")
    }
}
