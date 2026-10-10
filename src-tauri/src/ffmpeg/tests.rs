use super::{command, models::*, probe, progress};
use std::path::Path;
fn info() -> MediaInfo {
    probe::parse(r#"{"format":{"duration":"600","format_name":"matroska,webm","bit_rate":"500000"},"streams":[{"index":0,"codec_type":"video","codec_name":"h264","profile":"High","width":640,"height":360,"avg_frame_rate":"24000/1001","pix_fmt":"yuv420p","bits_per_raw_sample":"8"},{"index":1,"codec_type":"audio","codec_name":"aac","sample_rate":"48000","channels":2,"channel_layout":"stereo"}]}"#,Path::new("C:/视频 测试/demo.mkv")).unwrap()
}
fn request(op: MediaOperation, format: OutputFormat) -> MediaRequest {
    MediaRequest {
        input_path: "C:/视频 测试/demo.mkv".into(),
        output_dir: "C:/视频 测试".into(),
        operation: op,
        output_format: format,
        ..Default::default()
    }
}
#[test]
fn probe_reads_json_tracks_metadata_unknowns_and_rejects_invalid() {
    let i = info();
    assert_eq!(i.duration, Some(600.));
    assert_eq!(i.videos.len(), 1);
    assert_eq!(i.audios.len(), 1);
    assert!((i.videos[0].fps.unwrap() - 23.976).abs() < 0.001);
    assert_eq!(i.videos[0].bit_depth, Some(8));
    let audio=probe::parse(r#"{"streams":[{"index":0,"codec_type":"audio","codec_name":"mp3"},{"index":1,"codec_type":"audio","codec_name":"aac"},{"index":2,"codec_type":"subtitle","codec_name":"subrip","tags":{"language":"zh","title":"中文"}}],"format":{"duration":"N/A"}}"#,Path::new("C:/a.mkv")).unwrap();
    assert!(audio.videos.is_empty());
    assert_eq!(audio.audios.len(), 2);
    assert_eq!(audio.subtitles[0].language.as_deref(), Some("zh"));
    assert_eq!(audio.duration, None);
    let silent = probe::parse(
        r#"{"streams":[{"index":0,"codec_type":"video","codec_name":"h264"}]}"#,
        Path::new("C:/silent.mp4"),
    )
    .unwrap();
    assert!(silent.audios.is_empty());
    assert!(probe::parse("bad", Path::new("C:/x")).is_err());
    assert!(probe::parse("{}", Path::new("C:/x")).is_err());
    let a = probe::arguments(Path::new("C:/视频 测试/a.mp4"));
    assert!(a.contains(&"-protocol_whitelist".into()));
    assert!(a.contains(&"file,pipe".into()));
    assert!(a.contains(&"-format_whitelist".into()));
    assert_eq!(a.last().unwrap(), "C:/视频 测试/a.mp4");
}
#[test]
fn all_five_operations_are_structured_and_never_overwrite() {
    for (op, f) in [
        (MediaOperation::Remux, OutputFormat::Mp4),
        (MediaOperation::Transcode, OutputFormat::Mp4),
        (MediaOperation::Trim, OutputFormat::Mp4),
        (MediaOperation::ExtractAudio, OutputFormat::Mp3),
        (MediaOperation::Screenshot, OutputFormat::Png),
    ] {
        let mut r = request(op, f);
        r.start = 10.5;
        r.end = Some(70.5);
        let c = command::build(&r, &info(), Path::new("C:/视频 测试/output.processing")).unwrap();
        assert!(c.args.contains(&"-n".into()));
        assert!(!c.args.contains(&"-y".into()));
        assert!(c.args.contains(&"-nostdin".into()));
        assert!(c.args.contains(&"-progress".into()));
        assert!(c.args.contains(&r.input_path));
        assert!(c.args.contains(&"-protocol_whitelist".into()));
        if op == MediaOperation::Remux {
            assert!(c.args.windows(2).any(|a| a == ["-c", "copy"]));
        }
        if op == MediaOperation::ExtractAudio {
            assert!(c.args.contains(&"libmp3lame".into()));
            assert!(c.args.contains(&"320k".into()));
        }
        if op == MediaOperation::Screenshot {
            assert!(c.args.windows(2).any(|a| a == ["-frames:v", "1"]));
        }
    }
}
#[test]
fn container_check_includes_every_track_and_copy_rejects_transforms() {
    let mut i = info();
    let r = request(MediaOperation::Remux, OutputFormat::Mp4);
    i.audios.push(AudioStreamInfo {
        codec: "vorbis".into(),
        ..Default::default()
    });
    assert!(command::build(&r, &i, Path::new("C:/out.mp4")).is_err());
    let mut r = request(MediaOperation::Transcode, OutputFormat::Webm);
    assert!(command::build(&r, &info(), Path::new("C:/out.webm")).is_err());
    r.output_format = OutputFormat::Mp4;
    r.video_codec = VideoCodec::Copy;
    r.height = Some(720);
    assert!(command::build(&r, &info(), Path::new("C:/out.mp4")).is_err());
    r.height = None;
    r.video_codec = VideoCodec::H264;
    r.audio_codec = AudioCodec::Opus;
    assert!(command::build(&r, &info(), Path::new("C:/out.mp4")).is_err());
}
#[test]
fn invalid_times_paths_options_and_audio_copy_are_rejected() {
    let mut r = request(MediaOperation::Trim, OutputFormat::Mp4);
    for (start, end) in [(-1., 5.), (5., 4.), (0., 601.), (f64::NAN, 20.)] {
        r.start = start;
        r.end = Some(end);
        assert!(command::build(&r, &info(), Path::new("C:/out.mp4")).is_err());
    }
    r = request(MediaOperation::Screenshot, OutputFormat::Png);
    r.start = 600.;
    assert!(command::build(&r, &info(), Path::new("C:/out.png")).is_err());
    r = request(MediaOperation::ExtractAudio, OutputFormat::Mp3);
    r.copy_audio = true;
    assert!(command::build(&r, &info(), Path::new("C:/out.mp3")).is_err());
    r = request(MediaOperation::Transcode, OutputFormat::Mp4);
    r.input_path = "https://example.com/a.mp4".into();
    assert!(command::build(&r, &info(), Path::new("C:/out.mp4")).is_err());
}
#[test]
fn progress_uses_microseconds_and_allows_unknown_speed_and_eta() {
    let p = progress::parse(
        "out_time_us=30000000\nspeed=2.5x\nprogress=continue",
        Some(60.),
    );
    assert_eq!(p.processed_time, Some(30.));
    assert_eq!(p.percent, Some(50.));
    assert_eq!(p.eta, Some(12.));
    let p = progress::parse("out_time_us=N/A\nspeed=N/A\nprogress=end", None);
    assert_eq!(p.percent, None);
    assert_eq!(p.eta, None);
    let p = progress::parse("out_time_us=90000000\nspeed=1x", Some(60.));
    assert!(p.percent.unwrap() < 100.);
}
