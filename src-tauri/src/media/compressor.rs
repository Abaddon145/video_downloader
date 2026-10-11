use crate::ffmpeg::models::{Quality, VideoCodec};
pub fn quality_value(codec: VideoCodec, quality: Quality) -> u8 {
    match (codec, quality) {
        (VideoCodec::Hevc, Quality::High) => 20,
        (VideoCodec::Hevc, Quality::Balanced) => 26,
        (VideoCodec::Hevc, Quality::Small) => 30,
        (_, Quality::High) => 18,
        (_, Quality::Balanced) => 23,
        _ => 28,
    }
}
