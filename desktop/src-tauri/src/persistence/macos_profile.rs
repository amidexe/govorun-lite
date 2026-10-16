//! Existing physical namespaces for ordinary and isolated Debug macOS profiles.
pub fn isolated(identity: &str) -> bool {
    identity == "com.govorun.app.testbackend"
}
pub fn account_service(identity: &str) -> String {
    format!(
        "{identity}.account.v{}",
        if isolated(identity) { 1 } else { 2 }
    )
}
pub fn llm_service(identity: &str) -> String {
    format!("{identity}.llm.v{}", if isolated(identity) { 1 } else { 2 })
}
pub fn llm_cleanup_services(identity: &str) -> Vec<String> {
    let mut services = vec![llm_service(identity)];
    if !isolated(identity) {
        services.extend([format!("{identity}.llm"), identity.into()]);
    }
    services
}
