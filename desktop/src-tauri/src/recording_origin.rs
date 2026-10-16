//! A capsule is a separate local surface with snapshot/cancel permission only.
pub fn allows(label: &str, url: &url::Url, development: bool) -> bool {
    if label != "recording"
        || url.path() != "/recording.html"
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return false;
    }
    let origin = (url.scheme(), url.host_str(), url.port());
    let bundled = matches!(
        origin,
        ("tauri", Some("localhost"), None) | ("http" | "https", Some("tauri.localhost"), None)
    );
    bundled || (development && origin == ("http", Some("127.0.0.1"), Some(1420)))
}
