pub fn interval(duration: f64) -> f64 {
    if duration < 60. {
        2.
    } else if duration <= 1800. {
        5.
    } else {
        30.
    }
}
pub fn validate_session(s: &str) -> Result<(), String> {
    if s.is_empty() || s.len() > 80 || !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        Err("预览会话无效".into())
    } else {
        Ok(())
    }
}
