use crate::{
    domain::AppResult,
    ffmpeg::{
        command::{build, BuiltCommand},
        models::*,
    },
};
use std::path::Path;
pub fn title(info: &MediaInfo) -> String {
    let v = info.videos.iter().find(|v| !v.attached_picture);
    format!(
        "{}  |  {} x {}",
        info.file_name
            .chars()
            .filter(|c| !c.is_control())
            .take(120)
            .collect::<String>(),
        v.and_then(|v| v.width).unwrap_or(0),
        v.and_then(|v| v.height).unwrap_or(0)
    )
}
pub fn build_sheet(r: &MediaRequest, info: &MediaInfo, out: &Path) -> AppResult<BuiltCommand> {
    if r.sheet_columns == 0
        || r.sheet_rows == 0
        || r.sheet_columns > 10
        || r.sheet_rows > 10
        || r.sheet_columns * r.sheet_rows > 100
        || !matches!(r.output_format, OutputFormat::Jpg | OutputFormat::Png)
    {
        return Err("联系表支持1至10行/列、最多100格、JPG/PNG".into());
    }
    let duration = info
        .duration
        .filter(|d| d.is_finite() && *d > 0.)
        .ok_or("无法确定视频时长")?;
    let count = r.sheet_columns * r.sheet_rows;
    let fps = count as f64 / duration;
    let mut base = r.clone();
    base.operation = MediaOperation::Screenshot;
    base.start = 0.;
    let mut c = build(&base, info, out)?;
    let filter=format!("scale=320:-2,drawtext=font='Microsoft YaHei':text='%{{pts\\:hms}}':x=8:y=h-th-8:fontsize=18:fontcolor=white:box=1:boxcolor=black@0.7,fps={fps:.9}:start_time=0,tile={}x{}:nb_frames={count}:padding=8:margin=8:color=0x191d23,pad=iw:ih+64:0:64:color=0x191d23,drawtext=font='Microsoft YaHei':textfile=sheet-title.txt:expansion=none:x=12:y=20:fontsize=20:fontcolor=white",r.sheet_columns,r.sheet_rows);
    c.args
        .splice(c.args.len() - 1..c.args.len() - 1, ["-vf".into(), filter]);
    c.duration = Some(duration);
    Ok(c)
}
