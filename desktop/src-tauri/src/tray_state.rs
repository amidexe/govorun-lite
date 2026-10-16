pub fn pixels(mut bird: Vec<u8>, test: bool, template: bool) -> Vec<u8> {
    if template { return bird; }
    let color = if test { [208, 137, 21] } else { [255, 255, 255] };
    for pixel in bird.chunks_exact_mut(4) { pixel[..3].copy_from_slice(&color); }
    if !test && bird.len() == 40 * 40 * 4 {
        // A white silhouette disappears in the light Windows overflow panel.
        // Outline its alpha mask so it remains legible on both system themes,
        // including when Explorer changes theme while the app is running.
        let source = bird.clone();
        for y in 0..40_i32 {
            for x in 0..40_i32 {
                let offset = ((y * 40 + x) * 4) as usize;
                let mut outline = 0_u8;
                for dy in -2_i32..=2 {
                    for dx in -2_i32..=2 {
                        let (nx, ny) = (x + dx, y + dy);
                        if dx * dx + dy * dy <= 4 && (0..40).contains(&nx) && (0..40).contains(&ny) {
                            outline = outline.max(source[((ny * 40 + nx) * 4 + 3) as usize]);
                        }
                    }
                }
                let foreground = source[offset + 3] as u32;
                let under = (outline as u32 * (255 - foreground) + 127) / 255;
                let alpha = foreground + under;
                if alpha > 0 {
                    let channel = ((255 * foreground + 40 * under) / alpha) as u8;
                    bird[offset..offset + 4].copy_from_slice(&[channel, channel, channel, alpha as u8]);
                }
            }
        }
    }
    // The test profile remains distinguishable at tray size without relying on color alone.
    if test && bird.len() == 40 * 40 * 4 {
        for y in 23..40 {
            for x in 23..40 {
                let t = (27..36).contains(&x) && (26..29).contains(&y)
                    || (30..33).contains(&x) && (29..37).contains(&y);
                let offset = (y * 40 + x) * 4;
                bird[offset..offset + 4].copy_from_slice(if t { &[255, 255, 255, 255] }
                    else { &[208, 137, 21, 255] });
            }
        }
    }
    bird
}

pub fn tooltip(name: &str, version: &str, action: Option<&str>) -> String {
    match action {
        Some(action) => format!("{name} {version} — {action}"),
        None => format!("{name} {version}"),
    }
}
