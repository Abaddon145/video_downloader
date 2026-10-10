use super::models::MediaProgress;
pub fn parse(block: &str, total: Option<f64>) -> MediaProgress {
    let value = |key: &str| {
        block
            .lines()
            .filter_map(|s| s.split_once('='))
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    };
    let finite = |s: &str| s.parse::<f64>().ok().filter(|v| v.is_finite() && *v >= 0.);
    let time = value("out_time_us")
        .and_then(finite)
        .map(|v| v / 1_000_000.);
    let speed = value("speed")
        .and_then(|s| finite(s.trim_end_matches('x')))
        .filter(|v| *v > 0.);
    let duration = total.filter(|v| v.is_finite() && *v > 0.);
    MediaProgress {
        processed_time: time,
        percent: time
            .zip(duration)
            .map(|(t, d)| (t / d * 100.).clamp(0., 99.9)),
        speed,
        eta: time
            .zip(duration)
            .zip(speed)
            .map(|((t, d), s)| ((d - t).max(0.) / s).ceil()),
    }
}
