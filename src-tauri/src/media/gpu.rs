use super::preset::HardwareAcceleration;use crate::ffmpeg::models::VideoCodec;
pub fn choose(mode:HardwareAcceleration,codec:VideoCodec,available:&[String])->Option<String>{
 if mode==HardwareAcceleration::Cpu||codec==VideoCodec::Copy{return None;}let prefix=if codec==VideoCodec::Hevc{"hevc"}else{"h264"};
 for (device,suffix) in [(HardwareAcceleration::Nvidia,"nvenc"),(HardwareAcceleration::Intel,"qsv"),(HardwareAcceleration::Amd,"amf")] {let name=format!("{prefix}_{suffix}");if (mode==HardwareAcceleration::Auto||mode==device)&&available.contains(&name){return Some(name);}}None
}
pub fn apply(args:&mut Vec<String>,encoder:&str,quality:u8){
 let mut i=0;while i+1<args.len(){if ["-crf","-preset"].contains(&args[i].as_str()){args.drain(i..i+2);}else{if args[i]=="-c:v"{args[i+1]=encoder.into();}i+=1;}}
 let n=args.len()-1;let options:Vec<String>=if encoder.ends_with("nvenc"){vec!["-rc".into(),"vbr".into(),"-cq".into(),quality.to_string(),"-b:v".into(),"0".into()]}else if encoder.ends_with("qsv"){vec!["-global_quality".into(),quality.to_string()]}else{vec!["-rc".into(),"cqp".into(),"-qp_i".into(),quality.to_string(),"-qp_p".into(),quality.to_string()]};args.splice(n..n,options);
}

#[cfg(test)]mod tests{use super::*;#[test]fn gpu_declared_but_unusable_falls_back_cpu_and_available_hardware_is_selected(){assert_eq!(choose(HardwareAcceleration::Nvidia,VideoCodec::H264,&[]),None);assert_eq!(choose(HardwareAcceleration::Cpu,VideoCodec::H264,&["h264_nvenc".into()]),None);assert_eq!(choose(HardwareAcceleration::Nvidia,VideoCodec::H264,&["h264_nvenc".into()]),Some("h264_nvenc".into()));}}
