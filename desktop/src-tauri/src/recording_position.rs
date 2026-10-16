//! OS geometry for the existing capsule: 24 points above the usable bottom edge.
pub(crate) fn bottom(usable_bottom: f64, usable_top: f64, dock_reserve: f64) -> f64 {
    (usable_bottom - 68. - dock_reserve).max(usable_top + 24.)
}

#[cfg(target_os = "macos")]
pub(crate) fn dock_reserve(usable_bottom: f64, screen_bottom: f64) -> f64 {
    use objc2::AnyThread;
    use objc2_foundation::{NSString, NSUserDefaults};
    let Some(defaults) = NSUserDefaults::initWithSuiteName(
        NSUserDefaults::alloc(), Some(&NSString::from_str("com.apple.dock"))) else { return 0.; };
    let orientation = defaults.stringForKey(&NSString::from_str("orientation"));
    if !defaults.boolForKey(&NSString::from_str("autohide"))
        || orientation.is_some_and(|value| matches!(value.to_string().as_str(), "left" | "right"))
        || screen_bottom - usable_bottom > 1. { return 0.; }
    if defaults.objectForKey(&NSString::from_str("tilesize")).is_none() { return 64.; }
    defaults.doubleForKey(&NSString::from_str("tilesize")).clamp(32., 128.)
}
