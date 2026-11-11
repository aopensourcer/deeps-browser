//! The Deeps mark: a tilted vortex of white rings around a dark hole.
//!
//! One SVG for the UI and pages, one procedural 64x64 RGBA bitmap for the
//! native window icon. No image assets, no build step.

pub const SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" fill="none" class="vortex">
<g transform="rotate(-22 32 32)" stroke="#fff" stroke-linecap="round">
<ellipse cx="32" cy="32" rx="27" ry="17.4" stroke-width="3.6" opacity=".97" pathLength="100" stroke-dasharray="81 19" stroke-dashoffset="-2"/>
<ellipse cx="32" cy="32" rx="22.4" ry="14.4" stroke-width="3.1" opacity=".85" pathLength="100" stroke-dasharray="79 21" stroke-dashoffset="-12"/>
<ellipse cx="32" cy="32" rx="18.1" ry="11.6" stroke-width="2.6" opacity=".71" pathLength="100" stroke-dasharray="77 23" stroke-dashoffset="-22"/>
<ellipse cx="32" cy="32" rx="14.1" ry="9.1" stroke-width="2.1" opacity=".57" pathLength="100" stroke-dasharray="75 25" stroke-dashoffset="-32"/>
<ellipse cx="32" cy="32" rx="10.5" ry="6.8" stroke-width="1.7" opacity=".43" pathLength="100" stroke-dasharray="73 27" stroke-dashoffset="-42"/>
<ellipse cx="32" cy="32" rx="7.3" ry="4.7" stroke-width="1.3" opacity=".3" pathLength="100" stroke-dasharray="71 29" stroke-dashoffset="-52"/>
</g>
<ellipse cx="32" cy="32" rx="5.1" ry="3.3" fill="#08080a"/>
</svg>"##;

/// Draws the window icon: dark rounded square, white vortex, black hole.
pub fn icon_rgba(size: u32) -> Vec<u8> {
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let s = size as f64;
    let center = s / 2.0;
    let radius = s / 2.0;

    // rings: (rx, ry, stroke, gap centre, gap half-width, brightness)
    let rings: [(f64, f64, f64, f64, f64, f64); 6] = [
        (0.86, 0.55, 0.105, -0.12, 0.34, 1.00),
        (0.71, 0.45, 0.088, 0.40, 0.30, 0.90),
        (0.575, 0.365, 0.072, 0.92, 0.27, 0.78),
        (0.445, 0.283, 0.058, 1.44, 0.24, 0.66),
        (0.330, 0.210, 0.046, 1.96, 0.21, 0.54),
        (0.228, 0.145, 0.034, 2.48, 0.18, 0.42),
    ];

    for y in 0..size {
        for x in 0..size {
            let mut acc = [0.0f64; 4];
            for sy in 0..3 {
                for sx in 0..3 {
                    let px = x as f64 + (sx as f64 + 0.5) / 3.0;
                    let py = y as f64 + (sy as f64 + 0.5) / 3.0;
                    let sample = pixel(px, py, center, radius, &rings);
                    for c in 0..4 {
                        acc[c] += sample[c];
                    }
                }
            }
            let i = ((y * size + x) * 4) as usize;
            for c in 0..4 {
                rgba[i + c] = (acc[c] / 9.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    rgba
}

fn pixel(
    x: f64,
    y: f64,
    center: f64,
    radius: f64,
    rings: &[(f64, f64, f64, f64, f64, f64)],
) -> [f64; 4] {
    let dx = x - center;
    let dy = y - center;

    // rounded square mask (radius 22% of the icon)
    let corner = radius * 0.22;
    let inset = radius - 1.0;
    let qx = (dx.abs() - (inset - corner)).max(0.0);
    let qy = (dy.abs() - (inset - corner)).max(0.0);
    let outside = (qx * qx + qy * qy).sqrt() - corner;
    let edge = ((0.5 - outside) / 1.1).clamp(0.0, 1.0);
    if edge <= 0.0 {
        return [0.0; 4];
    }

    let mut r: f64 = 17.0;
    let mut g: f64 = 18.0;
    let mut b: f64 = 23.0;
    let mut a: f64 = 255.0 * edge;

    if dx * dx + dy * dy < (radius * 0.115).powi(2) {
        r = 6.0;
        g = 6.0;
        b = 8.0;
    } else {
        for (rx, ry, stroke, gap, gap_half, bright) in rings {
            let nx = dx / (radius * rx);
            let ny = dy / (radius * ry);
            let d = (nx * nx + ny * ny).sqrt();
            let width = stroke * 0.52;
            if (d - 1.0).abs() < width {
                let angle = ny.atan2(nx);
                let mut delta = angle - gap;
                while delta > std::f64::consts::PI {
                    delta -= std::f64::consts::TAU;
                }
                while delta < -std::f64::consts::PI {
                    delta += std::f64::consts::TAU;
                }
                if delta.abs() > *gap_half {
                    let falloff = 1.0 - ((d - 1.0).abs() / width).powi(2);
                    let value = (bright * falloff * 255.0 * edge).clamp(0.0, 255.0);
                    // White marks with a cool shadow beside them.
                    r = r.max(value * 0.93);
                    g = g.max(value * 0.95);
                    b = b.max(value);
                    a = a.max(value + 30.0 * edge);
                }
            }
        }
    }

    [r, g, b, a.clamp(0.0, 255.0)]
}
