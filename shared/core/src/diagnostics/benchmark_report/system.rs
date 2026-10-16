use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "platform", rename_all = "lowercase", deny_unknown_fields)]
pub enum SystemFacts {
    #[serde(rename_all = "camelCase")]
    Macos {
        version: String,
        build_id: String,
        os_version: String,
        machine: String,
        cpu: String,
        cpu_threads: u32,
        memory_mi_b: u64,
        gpu: String,
    },
    #[serde(rename_all = "camelCase")]
    Windows {
        version: String,
        build_id: String,
        os_version: String,
        architecture: String,
        cpu: String,
        cpu_threads: u32,
        memory_mi_b: Option<u64>,
        adapters: Vec<Adapter>,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Adapter {
    pub name: String,
    pub driver: String,
}
impl SystemFacts {
    pub(super) fn checked(&self) -> bool {
        let valid = |text: &str| super::text(text, 160);
        match self {
            Self::Macos {
                version,
                build_id,
                os_version,
                machine,
                cpu,
                cpu_threads,
                gpu,
                ..
            } => {
                [version, build_id, os_version, machine, cpu, gpu]
                    .into_iter()
                    .all(|text| valid(text))
                    && *cpu_threads > 0
            }
            Self::Windows {
                version,
                build_id,
                os_version,
                architecture,
                cpu,
                cpu_threads,
                adapters,
                ..
            } => {
                [version, build_id, os_version, architecture, cpu]
                    .into_iter()
                    .all(|text| valid(text))
                    && *cpu_threads > 0
                    && adapters.len() <= 4
                    && adapters
                        .iter()
                        .all(|adapter| valid(&adapter.name) && valid(&adapter.driver))
            }
        }
    }
    pub(super) fn render(&self) -> String {
        let engine = "Движок: transcribe.cpp 0.2.1 · GigaAM v3 E2E RNNT · GGUF Q8_0";
        match self {
            Self::Macos {version,build_id,os_version,machine,cpu,cpu_threads,memory_mi_b,gpu}=>format!(
                "Говорун {version} · {build_id}\nmacOS {os_version}\nMac: {machine}\nCPU: {cpu} · потоков: {cpu_threads}\nПамять: {memory_mi_b} MiB\nGPU: {gpu}\nДрайвер Metal: входит в установленную macOS\nДоступно в приложении: CPU, Metal при поддержке устройства, автоматический переход на CPU\n{engine}"),
            Self::Windows {version,build_id,os_version,architecture,cpu,cpu_threads,memory_mi_b,adapters}=> {
                let ram=memory_mi_b.map(|ram|format!("{ram} MiB")).unwrap_or_else(||"не определена".into());
                let adapters=if adapters.is_empty() {"сведения недоступны".into()} else {adapters.iter().map(|a|format!("{} · драйвер {}",a.name,a.driver)).collect::<Vec<_>>().join("; ")};
                format!("Служба Говоруна {version} · {build_id}\nWindows: {os_version} · {architecture}\nCPU: {cpu} · потоков: {cpu_threads}\nПамять: {ram}\nВидеоадаптеры по сведениям Windows: {adapters}\nДоступно в приложении: CPU, Vulkan при поддержке устройства, автоматический переход на CPU\nАктивный ускоритель определяется результатом теста; список Windows не подтверждает работу Vulkan.\n{engine}")
            }
        }
    }
    pub(super) fn gpu_api(&self) -> &'static str {
        match self {
            Self::Macos { .. } => "Metal",
            Self::Windows { .. } => "Vulkan",
        }
    }
}
