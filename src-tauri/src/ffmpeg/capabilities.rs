use crate::{domain::AppResult, ffmpeg::runner::MediaService};
pub const ENCODERS: [&str; 6] = [
    "h264_nvenc",
    "hevc_nvenc",
    "h264_qsv",
    "hevc_qsv",
    "h264_amf",
    "hevc_amf",
];
pub fn declared(text: &str) -> Vec<String> {
    ENCODERS
        .into_iter()
        .filter(|n| {
            text.lines()
                .any(|l| l.split_whitespace().nth(1) == Some(*n))
        })
        .map(str::to_string)
        .collect()
}
impl MediaService {
    pub fn capabilities(&self, session: &str) -> AppResult<Vec<String>> {
        self.ensure_ready()?;
        let mut cached = self.gpu_cache.lock().unwrap();
        if let Some(c) = cached.as_ref() {
            return Ok(c.clone());
        }
        let listed = self.auxiliary(session, &["-hide_banner".into(), "-encoders".into()], 10)?;
        let mut usable = Vec::new();
        for encoder in declared(&listed) {
            let args = [
                "-hide_banner",
                "-nostdin",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=black:s=128x128:r=1",
                "-frames:v",
                "1",
                "-an",
                "-pix_fmt",
                "nv12",
                "-c:v",
                &encoder,
                "-f",
                "null",
                "-",
            ]
            .map(str::to_string);
            if self.auxiliary(session, &args, 10).is_ok() {
                usable.push(encoder);
            }
        }
        *cached = Some(usable.clone());
        Ok(usable)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn encoder_parser_does_not_confuse_descriptions_with_names() {
        assert_eq!(
            declared(
                " V....D h264_nvenc NVIDIA H264
 V....D libx264 mentions hevc_qsv
"
            ),
            vec!["h264_nvenc"]
        );
    }
}
