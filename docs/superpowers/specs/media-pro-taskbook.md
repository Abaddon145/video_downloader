# 映流 video_downloader v0.3.0
# FFmpeg Media Pro + Visual Trim Editor 开发任务

项目：

GitHub:
Abaddon145/video_downloader


当前技术：

- Tauri 2
- Rust
- React
- TypeScript
- Vite
- yt-dlp
- FFmpeg
- ffprobe


当前状态：

已经完成 v0.2 FFmpeg Core：

✓ ffprobe媒体分析

✓ FFmpeg任务系统

✓ 无损封装 Remux

✓ 视频格式转换

✓ 基础视频裁剪

✓ 音频提取

✓ 视频截图


现在开发：

版本：

v0.3.0

名称：

FFmpeg Media Pro


---

# 一、v0.3总体目标


将映流从：

视频下载器


升级为：


视频下载

+

专业媒体处理工具


最终定位：

类似：

yt-dlp

+

LosslessCut

+

HandBrake

+

本地媒体管理


---

# 二、重要开发原则


## 1. 不改变现有下载功能


必须保证：

- yt-dlp下载
- Cookie
- 代理
- 播放列表
- 下载队列
- 断点续传
- 托盘
- 历史记录


全部正常。


---

## 2. FFmpeg继续采用CLI方式


禁止：

直接集成：

libavcodec

libavformat

FFmpeg C API


继续使用：

ffmpeg.exe

ffprobe.exe


由Rust统一调用。


---

## 3. 禁止前端直接执行FFmpeg命令


禁止：

前端传：

ffmpeg -i xxx


必须：

React

↓

结构化参数

↓

Rust验证

↓

生成FFmpeg参数

↓

执行


---

# 三、v0.3整体功能规划


## P0核心功能


必须完成：


1. Visual Trim Editor
可视化视频裁剪器


2. 视频压缩系统


3. 媒体处理预设


4. 下载完成自动处理


5. GPU编码检测架构



---

## P1功能


完成：

6. 字幕处理系统

7. 批量媒体处理

8. 视频抽帧

9. 视频联系表

10. 封面管理

11. 元数据管理



---

## P2预留


12. 媒体库升级

13. AI媒体处理

14. 自动素材分析


---

# 四、功能1：Visual Trim Editor
# 可视化视频裁剪器


参考：

LosslessCut


目标：

达到专业剪辑软件基础体验。


---

## 4.1 页面


新增：

媒体工具

↓

视频裁剪


页面结构：


视频预览窗口


时间显示


播放控制


缩略图Timeline


入点/出点控制


裁剪设置


处理按钮


---

## 4.2 视频预览窗口


新增组件：

src/components/media/editor/


VideoPreview.tsx


支持：


播放

暂停

当前时间

总时长


显示：

00:01:25.300 / 00:10:00


控制：


-5秒

-1秒

+1秒

+5秒


逐帧：

上一帧

下一帧


根据fps计算。


---

## 4.3 Timeline时间轴


新增：

TimelineEditor.tsx


参考：

LosslessCut


显示：


0s ---------------- 10min


支持：

左控制点：

In Point


右控制点：

Out Point



拖动：

实时修改：


startTime

endTime


---

## 4.4 视频缩略图时间轴


新增：

ThumbnailTimeline.tsx


使用FFmpeg生成缩略图。


例如：


ffmpeg

-i input.mp4

-vf fps=1/5

thumbnail_%04d.jpg



规则：


视频<60秒：

每2秒


60秒-30分钟：

每5秒


>30分钟：

每30秒


显示：


|图|图|图|图|图|


---

## 4.5 时间双向同步


必须支持：


Timeline拖动

同步：

开始时间

结束时间


输入框修改

同步：

Timeline位置


---

## 4.6 快捷键


参考LosslessCut。


支持：


Space

播放/暂停


I

设置入点


O

设置出点


J

倒放


K

暂停


L

播放


左右键：

移动1秒


Shift+左右：

逐帧移动


---

## 4.7 片段预览


增加：


预览裁剪片段


逻辑：


播放：

startTime


到：

endTime


自动停止。


---

## 4.8 裁剪模式


保留两种：


### 极速无损裁剪


FFmpeg：

-ss

-to

-c copy


特点：

最快

不重新编码


提示：

关键帧影响切点。


---


### 精确裁剪


重新编码：


视频：

libx264


音频：

aac


保证：

时间准确。


---

# 五、功能2：视频压缩系统


目标：

类似HandBrake。


---

## 简单模式


用户选择：


最高质量


平衡


更小体积



自动生成：

CRF参数。



---

## 高级模式


支持：


编码：

H264

H265

AV1（预留）


质量：

CRF


分辨率：

2160P

1440P

1080P

720P


FPS：

保持

30

60



---

# 六、功能3：GPU硬件编码


新增：

ffmpeg/capabilities.rs


检测：

ffmpeg -encoders


支持：


NVIDIA：

h264_nvenc

hevc_nvenc


Intel：

h264_qsv

hevc_qsv


AMD：

h264_amf

hevc_amf


---

设置增加：


编码加速


自动


CPU


NVIDIA


Intel


AMD


---

规则：

如果GPU不可用：

自动回退CPU。


不能直接失败。


---

# 七、功能4：媒体处理预设


新增：

Media Preset


支持保存：


## 手机视频


MP4

H264

1080P

AAC


---

## 网页发布


MP4

H264

较小码率


---

## AI素材


高质量

保持原始

生成缩略图


---

## 归档


MKV

HEVC

最高质量


---

数据结构：


MediaPreset {


id

name

operation

videoCodec

audioCodec

resolution

quality

hardwareAcceleration


}



---

# 八、功能5：下载完成自动处理


新增：


下载完成动作


选项：


不处理


自动转换MP4


自动压缩


自动字幕


自动生成封面



流程：


URL

↓

yt-dlp下载

↓

完成

↓

FFmpeg处理

↓

最终文件



---

# 九、功能6：字幕处理系统


支持：


输入：

SRT

ASS

VTT



输出：

SRT

ASS

VTT



功能：


## 外挂字幕


视频

+

字幕轨



## 硬字幕烧录


字幕直接进入画面。


支持：

字体

大小

位置

颜色


---

# 十、功能7：批量媒体处理


支持：


拖入文件夹


批量：


转换

压缩

提取音频

生成截图



复用：

MediaTask


新增：

BatchMediaTask



---

# 十一、功能8：视频抽帧


新增：

Extract Frames


支持：


按时间：

每5秒


按数量：

生成20张


按FPS：

5fps



输出：

frames/


---

# 十二、功能9：视频联系表


自动生成：

Storyboard


例如：


+----+----+----+

|01  |02  |03  |

+----+----+----+

|04  |05  |06  |

+----+----+----+


包含：

时间码

文件名

分辨率


---

# 十三、功能10：封面管理


支持：


提取封面


视频

↓

jpg/png


设置封面


批量生成缩略图



---

# 十四、功能11：元数据管理


读取：

ffprobe metadata


支持修改：


视频：

标题

作者

日期

描述


音乐：

Artist

Album

Cover

Genre



---

# 十五、Rust模块结构


不要继续扩大service.rs。


新增：


src-tauri/src/media/


结构：


media/

├── trim/

│   ├── mod.rs

│   ├── preview.rs

│   ├── thumbnail.rs

│   ├── timeline.rs

│   └── processor.rs


├── compressor.rs

├── gpu.rs

├── subtitle.rs

├── batch.rs

├── preset.rs

├── metadata.rs

├── thumbnail.rs

└── contact_sheet.rs



---

# 十六、前端结构


新增：


src/components/media/


editor/

VideoPreview.tsx

PlaybackControls.tsx

TimelineEditor.tsx

ThumbnailTimeline.tsx

RangeSelector.tsx

TrimSettings.tsx



compress/

CompressorPanel.tsx


subtitle/

SubtitlePanel.tsx


batch/

BatchPanel.tsx


preset/

PresetManager.tsx



---

# 十七、文件安全


必须继承现有机制。


禁止覆盖。


例如：


输入：

video.mp4


输出：

video_processed.mp4



存在：

video_processed(1).mp4



处理临时文件：


.video-downloader/media-temp/


完成：

验证文件

移动到最终位置。



---

# 十八、任务系统


统一：

MediaTask


状态：


queued

processing

completed

failed

cancelled



显示：

进度

速度

ETA

错误


支持：

取消

重试

打开文件

打开目录


---

# 十九、测试要求


必须通过：


npm test


npm run build


cargo test --manifest-path src-tauri/Cargo.toml


npm run tauri -- build --bundles nsis



---

测试：

## 裁剪

极速模式

精确模式


## 压缩

H264

H265


## GPU

无GPU环境回退


## 字幕

SRT

ASS


## 批处理

多个视频


## 文件保护

不能覆盖


## 取消任务

FFmpeg进程退出



---

# 二十、开发顺序


严格执行：


Step1

分析v0.2 FFmpeg模块。


Step2

实现Visual Trim Editor。


Step3

实现视频压缩。


Step4

实现Media Preset。


Step5

实现下载完成自动处理。


Step6

实现GPU检测。


Step7

实现字幕系统。


Step8

实现批量处理。


Step9

实现抽帧、联系表、封面。


Step10

测试和构建。



---

# 二十一、最终目标


完成v0.3后：


映流 =


视频下载器


+

LosslessCut级裁剪


+

HandBrake级压缩


+

FFmpeg媒体工具箱


+

自动化素材处理平台



适用于：

- UE5项目素材管理
- AI视频素材处理
- 赛事视频制作
- 日常视频管理



---

# Codex执行要求


请直接修改当前仓库。


要求：

1. 不破坏现有下载功能。

2. 不复制第三方源码。

3. 参考LosslessCut交互。

4. 所有FFmpeg调用必须经过Rust。

5. 所有功能模块化。

6. 保持当前UI风格。

7. 完成后提供：

- 修改文件列表
- 新增模块说明
- 功能完成情况
- 测试结果
- 构建结果
- 已知限制
- 下一阶段建议


不要只提供方案。

直接实现。