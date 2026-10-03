//! JavaScript number semantics the engine depends on (it was translated from TypeScript).

/// `Math.round`: halves round toward +∞
pub fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// `Math.max(seed, ...xs)`
pub fn max(seed: f64, xs: impl IntoIterator<Item = f64>) -> f64 {
    xs.into_iter().fold(seed, f64::max)
}

/// `Math.min(seed, ...xs)`
pub fn min(seed: f64, xs: impl IntoIterator<Item = f64>) -> f64 {
    xs.into_iter().fold(seed, f64::min)
}

/// a number as JavaScript prints it (`String(x)`) for the magnitudes layouts use
pub fn num(x: f64) -> String {
    if x == 0.0 {
        "0".to_string()
    } else {
        format!("{x}")
    }
}
