pub fn require_update_feed(packaged: bool, feed: Option<&str>) -> Result<(), &'static str> {
    if !packaged { return Ok(()); }
    if matches!(feed, Some("https://govorun.club/updates/tauri/test.json" | "https://govorun.club/updates/tauri/stable.json")) {
        Ok(())
    } else {
        Err("packaged application requires a standard Tauri update channel")
    }
}
