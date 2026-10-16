//! Shared state with OS-specific tray pixel rendering.
pub use govorun_core::features::attention_state::*;
// Only actionable dictation failures and available updates receive a mark.
// The monochrome exclamation remains recognizable without colour.
#[cfg(any(not(target_os = "macos"), test))]
pub fn mark(mut pixels: Vec<u8>) -> Vec<u8> {
    if pixels.len() != 40 * 40 * 4 { return pixels; }
    badge(&mut pixels, 40, 30, 0, 10, 12);
    pixels
}

#[cfg(any(windows, test))]
pub fn overlay() -> Vec<u8> {
    let mut pixels = vec![0; 16 * 16 * 4];
    badge(&mut pixels, 16, 2, 1, 12, 14);
    pixels
}

#[cfg(any(not(target_os = "macos"), test))]
fn badge(pixels: &mut [u8], width: usize, left: usize, top: usize, w: usize, h: usize) {
    let symbol = |x: i32, y: i32| {
        (w as i32 / 2 - 1..=w as i32 / 2 + 1).contains(&x) &&
            ((1..h as i32 - 5).contains(&y) || (h as i32 - 3..h as i32 - 1).contains(&y))
    };
    for y in 0..h { for x in 0..w {
        let foreground = symbol(x as i32, y as i32);
        let outline = (-1..=1).any(|dy| (-1..=1).any(|dx| symbol(x as i32 + dx, y as i32 + dy)));
        if !outline { continue; }
        let channel = if foreground { 255 } else { 32 };
        let offset = ((top + y) * width + left + x) * 4;
        pixels[offset..offset + 4].copy_from_slice(&[channel, channel, channel, 255]);
    } }
}
