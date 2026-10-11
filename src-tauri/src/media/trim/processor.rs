// Export always operates on the original source, never on the preview proxy.
pub fn validate_duration(duration: Option<f64>) -> Result<f64,String> {
 duration.filter(|d|d.is_finite()&&*d>0.).ok_or_else(||"无法确定视频时长".into())
}
