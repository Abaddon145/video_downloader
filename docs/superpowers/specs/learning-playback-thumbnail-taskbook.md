# 映流 v0.4.1
# Learning Playback & Thumbnail Enhancement
# 多格式播放 + 课程缩略图增强
# Codex 执行文档

---

# 一、项目背景

项目：

映流（video_downloader）

当前稳定基础：

```text
v0.4.0 Learning Library
```

目标版本：

```text
v0.4.1
```

版本名称：

```text
Learning Playback & Thumbnail Enhancement
```

中文名称：

```text
学习库多格式播放与课程缩略图增强
```

---

# 二、版本目标

当前映流已经具备：

```text
视频下载
Playlist解析
学习库
分类
系列
课程
本地视频导入
在线播放列表导入
播放进度
继续学习
自动下一课
完成状态
时间点笔记
系列更新
FFmpeg Media Pro
视频裁剪
压缩
字幕
批处理
GPU检测
```

但是当前学习库还存在两个重要体验问题：

```text
1. WebView2原生播放格式有限
2. 课程列表缺少视频缩略图
```

v0.4.1 需要同时解决。

最终目标：

```text
用户导入各种视频
↓
映流自动判断播放方式
↓
能直接播就直接播放
↓
不能直接播则快速Remux
↓
仍不兼容则自动生成兼容代理
↓
自动播放
```

并且：

```text
每节课程
↓
自动获得缩略图
↓
显示时长
↓
显示观看进度
↓
显示完成状态
↓
当前播放高亮
```

最终将学习库升级为：

```text
真正可长期使用的本地视频学习中心
```

---

# 三、本版本核心范围

本版本包含两大模块：

```text
A. Multi-Format Playback
B. Lesson Thumbnail System
```

---

# 四、P0 必须完成

## 多格式播放

```text
MP4
MOV
MKV
AVI
WMV
ASF
WebM
FLV
MPG
MPEG
TS
M2TS
MTS
3GP
3G2
OGV
```

只要：

```text
FFmpeg可以正确解码
```

就应尽量支持在学习播放器中播放。

---

## 课程缩略图

每节课程必须支持：

```text
本地课程缩略图
源站Playlist缩略图
本地视频自动抽帧
观看进度条
时长角标
已完成状态
当前播放高亮
下载状态
文件缺失状态
```

---

# 五、重要架构原则

## 1. 不重写学习库

必须保留：

```text
Category
Series
LearningItem
PlaybackProgress
VideoNote
Series Update
Download Binding
SQLite
```

---

## 2. 不重写 FFmpeg

必须复用当前已有：

```text
src-tauri/src/ffmpeg/
```

包括现有：

```text
probe
runner
command
output
capabilities
Windows Job Object
GPU能力检测
```

禁止重新创建第二套：

```text
FFmpegRunner
ProcessManager
MediaProbe
```

---

## 3. 播放兼容和视频原文件分离

永远保证：

```text
原始文件
≠
播放代理文件
```

例如：

```text
lesson01.wmv
```

仍然保持原样。

播放器使用：

```text
lesson01.proxy.mp4
```

不能修改原文件。

---

## 4. 学习进度绑定 LearningItem

播放：

```text
原文件
或
Proxy
```

都必须继续写：

```text
LearningItem.id
```

对应的：

```text
PlaybackProgress
```

不能把学习进度绑到：

```text
proxy路径
```

---

## 5. Thumbnail 与 Player 解耦

缩略图：

```text
失败
生成中
缓存失效
```

都不能阻止：

```text
课程播放
```

---

# 六、开发前检查

开始开发前先检查实际最新代码：

```text
src-tauri/src/learning/models.rs
src-tauri/src/learning/player.rs
src-tauri/src/learning/importer.rs
src-tauri/src/learning/download.rs
src-tauri/src/learning/repository.rs
src-tauri/src/learning/service.rs
src-tauri/src/learning/db.rs
src-tauri/src/learning/schema-*.sql

src/features/learning/LearningLibraryPage.tsx
src/features/learning/LearningPlayer.tsx
src/features/learning/ImportDialog.tsx

src-tauri/src/ffmpeg/
src-tauri/src/media/
```

确认：

```text
thumbnail_path
PlaylistEntry.thumbnail
现有Media Pro缩略图能力
现有代理预览能力
现有GPU转码
现有Subtitle处理
```

已经存在的功能直接复用。

不要重复造轮子。

---

# 七、第一部分
# Multi-Format Playback

---

# 八、播放兼容总体架构

采用三级播放策略：

```text
Local Media
↓
ffprobe
↓
Playback Compatibility Planner
↓
┌─────────────────────┐
│ Level 1 Direct      │
└─────────────────────┘
↓
不兼容
↓
┌─────────────────────┐
│ Level 2 Remux       │
└─────────────────────┘
↓
仍不兼容
↓
┌─────────────────────┐
│ Level 3 Transcode   │
└─────────────────────┘
↓
Compatible Proxy
```

---

# 九、PlaybackMode

新增：

```rust
enum PlaybackMode {
    Direct,
    Remux,
    Transcode,
    Unsupported,
}
```

建议：

```rust
pub struct PlaybackPlan {
    pub mode: PlaybackMode,
    pub source_path: String,
    pub playback_path: Option<String>,
    pub cached: bool,
    pub reason: Option<String>,
}
```

---

# 十、Level 1：Direct Playback

以下组合优先直接播放：

```text
MP4 + H.264 + AAC
MP4 + H.264 + MP3
WebM + VP8 + Opus
WebM + VP9 + Opus
部分 MOV + H.264 + AAC
```

实际判断：

必须结合：

```text
ffprobe
```

不要只按：

```text
扩展名
```

判断。

---

# 十一、Level 2：Remux

如果：

```text
视频编码浏览器兼容
+
音频编码兼容
+
仅Container不兼容
```

使用：

```text
Remux
```

例如：

```text
MKV
Video: H.264
Audio: AAC
```

转换为：

```text
MP4
Video: copy
Audio: copy
```

FFmpeg：

```text
-c:v copy
-c:a copy
-movflags +faststart
```

优点：

```text
速度快
无画质损失
CPU占用极低
```

---

# 十二、Level 3：Transcode Proxy

以下类型通常需要代理：

```text
WMV
VC-1
WMV3
WMA
部分AVI
MPEG-2
部分FLV
HEVC不兼容环境
不兼容音频编码
```

统一目标：

```text
Container:
MP4

Video:
H.264

Audio:
AAC
```

推荐：

```text
CRF 20~23
AAC 192k
movflags +faststart
```

---

# 十三、WMV 重点支持

必须支持：

```text
.wmv
.asf
```

例如：

```text
Container: ASF
Video: WMV3 / VC-1
Audio: WMA
```

自动：

```text
WMV
↓
FFmpeg
↓
H.264 + AAC MP4
↓
Learning Player
```

用户不需要手动转换。

---

# 十四、导入器升级

当前导入器不能仅按扩展名判断：

```text
可播放
```

正确逻辑：

```text
文件存在
↓
ffprobe
↓
存在Video Stream
↓
允许进入Learning Library
```

扩展名只是：

```text
文件选择器过滤
```

不是最终判断依据。

---

# 十五、推荐支持的导入扩展名

至少：

```text
mp4
m4v
mov
mkv
avi
wmv
asf
webm
flv
mpg
mpeg
ts
m2ts
mts
3gp
3g2
ogv
```

未来新增：

```text
VOB
MXF
RMVB
```

不需要重构播放器。

---

# 十六、建议新增后端模块

新增：

```text
src-tauri/src/learning/compatibility.rs
```

负责：

```text
probe媒体
判断Direct
判断Remux
判断Transcode
生成PlaybackPlan
缓存检查
Proxy生命周期
```

不要继续把大量逻辑堆到：

```text
player.rs
```

---

# 十七、Player Prepare Flow

现有：

```text
learning_prepare_player
```

优先继续作为统一入口。

内部：

```text
LearningItem
↓
验证file_path
↓
ffprobe
↓
Playback Compatibility Planner
↓
Direct / Remux / Transcode
↓
返回PlayerMedia
```

前端不需要理解：

```text
WMV3
VC1
H264
AAC
```

等技术细节。

---

# 十八、播放兼容缓存目录

新增：

```text
AppData/
映流/
learning-cache/
playback/
```

建议：

```text
learning-cache/
playback/
└ {cache-key}/
  ├ video.mp4
  └ info.json
```

---

# 十九、Playback Cache Key

建议根据：

```text
normalized source path
file size
mtime
codec
container
target profile
```

生成。

如果：

```text
原文件变化
```

旧 Proxy 自动失效。

---

# 二十、播放代理禁止重复生成

同一源视频：

```text
连续点击三次
```

不能产生：

```text
ProxyJob 1
ProxyJob 2
ProxyJob 3
```

必须：

```text
任务去重
```

其他请求复用：

```text
正在运行的Proxy任务
```

---

# 二十一、自动兼容播放 UX

当前不要继续以：

```text
生成兼容预览
```

作为主要操作。

点击 WMV 时：

```text
正在准备视频…
```

然后：

Direct：

```text
立即播放
```

Remux：

```text
正在优化播放兼容性…
```

Transcode：

```text
正在准备兼容播放…
45%
预计剩余 00:18
```

完成：

```text
自动播放
```

---

# 二十二、用户高级操作

在：

```text
⋯
```

或高级按钮中保留：

```text
重新生成兼容版本
```

普通用户不需要自己判断：

```text
什么时候转换
```

---

# 二十三、GPU 加速

Transcode 优先复用 v0.3 Media Pro 的：

```text
GPU Detection
```

支持：

```text
NVIDIA NVENC
Intel QSV
AMD AMF
CPU
```

策略：

```text
Auto
↓
可用GPU
↓
GPU转码
↓
失败
↓
CPU H.264
```

必须保持：

```text
GPU失败 → CPU自动回退
```

---

# 二十四、长视频

第一版允许：

```text
完整Proxy生成后播放
```

但需要：

```text
进度显示
取消按钮
```

后续版本可升级：

```text
HLS
fragmented MP4
边转码边播放
```

v0.4.1 不强制做流式转码。

---

# 二十五、Proxy 状态

建议：

```text
direct
preparing
remuxing
transcoding
ready
failed
unsupported
```

前端只显示友好文案：

```text
原始视频
正在准备兼容播放
兼容版本
无法播放
```

---

# 二十六、播放失败

如果：

```text
FFmpeg也无法解码
```

显示：

```text
无法播放此视频
```

并提供：

```text
打开文件位置
重新定位文件
查看技术详情
```

禁止无限循环转码。

---

# 二十七、字幕兼容

支持外部：

```text
SRT
ASS
SSA
VTT
```

如果浏览器不支持：

```text
SRT
ASS
SSA
```

自动转换：

```text
VTT
```

缓存。

---

# 二十八、内嵌字幕

如果：

```text
MKV
MP4
```

存在文本字幕轨：

允许：

```text
提取
↓
转换VTT
↓
加载到Learning Player
```

不要为了字幕切换：

```text
重新编码整个视频
```

---

# 二十九、播放进度保持

无论：

```text
Direct
Remux
Transcode
```

学习进度都继续保存：

```text
LearningItem.id
```

例如：

```text
WMV
↓
Proxy MP4
↓
用户看到18:32
↓
PlaybackProgress.position = 18:32
```

以后：

```text
Proxy被清理
重新生成
```

仍恢复：

```text
18:32
```

---

# 三十、播放缓存设置

学习库设置新增：

```text
播放兼容缓存
```

显示：

```text
当前占用：18.6 GB
```

设置：

```text
自动清理30天未使用代理

最大缓存：
50 GB
```

按钮：

```text
清理未使用兼容缓存
```

不要自动删除：

```text
正在使用中的Proxy
```

---

# 三十一、第二部分
# Lesson Thumbnail System

---

# 三十二、课程列表目标

当前：

```text
▶ Title        Duration       Actions
```

升级：

```text
[Thumbnail]  Title
             Progress
             State
                           Actions
```

---

# 三十三、缩略图来源优先级

统一：

```text
1 manual
2 remote
3 generated
4 series
5 placeholder
```

具体：

```text
用户手动缩略图
↓
源站缩略图缓存
↓
本地视频抽帧
↓
Series Cover
↓
Placeholder
```

---

# 三十四、复用现有 thumbnail_path

如果：

```text
series_items.thumbnail_path
```

已经存在：

必须复用。

不要重复增加：

```text
thumb
image
preview
cover
```

等同义字段。

---

# 三十五、数据库字段

如有必要增加：

```text
thumbnail_source
source_thumbnail_url
thumbnail_key
```

建议：

```text
thumbnail_source:
manual
remote
generated
series
placeholder
```

---

# 三十六、数据库迁移

假设当前：

```text
schema_version = 4
```

升级：

```text
4 → 5
```

示例：

```sql
ALTER TABLE series_items
ADD COLUMN thumbnail_source TEXT;

ALTER TABLE series_items
ADD COLUMN source_thumbnail_url TEXT;

ALTER TABLE series_items
ADD COLUMN thumbnail_key TEXT;

UPDATE schema_version SET version=5;
```

只增加真实使用字段。

---

# 三十七、Thumbnail 缓存目录

使用：

```text
AppData/
映流/
learning-cache/
thumbnails/
```

不要保存到：

```text
原课程目录
```

---

# 三十八、Thumbnail 标准尺寸

统一：

```text
320 × 180
```

格式：

```text
JPEG
```

推荐质量：

```text
80~85
```

---

# 三十九、课程列表显示尺寸

宽屏：

```text
128 × 72
```

窄屏：

```text
96 × 54
```

保持：

```text
16:9
```

---

# 四十、源站 Thumbnail

现有 PlaylistEntry：

```text
thumbnail
```

流程：

```text
PlaylistEntry.thumbnail
↓
保存source_thumbnail_url
↓
后台下载
↓
标准化
↓
保存本地thumbnail_path
```

---

# 四十一、远程 Thumbnail 必须缓存

禁止长期直接依赖：

```text
https://...
```

本地缓存后：

```text
离线也显示
```

---

# 四十二、源站 Thumbnail 下载失败

流程：

```text
Remote失败
↓
本地视频存在
↓
FFmpeg抽帧
↓
Series Cover
↓
Placeholder
```

不得影响：

```text
Item创建
```

---

# 四十三、本地视频自动抽帧

无源站缩略图：

```text
file_path
↓
FFmpeg
↓
thumbnail
```

支持：

```text
MP4
MOV
MKV
AVI
WMV
ASF
MPEG
TS
M2TS
FLV
WebM
...
```

只要：

```text
FFmpeg可解码
```

---

# 四十四、抽帧时间

推荐：

```text
duration × 12%
```

特殊：

```text
duration >= 30s
→ 12%

5s <= duration < 30s
→ 25%

duration < 5s
→ 50%
```

禁止默认第一帧。

---

# 四十五、非16:9画面

保持原始比例。

建议：

```text
scale=320:180:force_original_aspect_ratio=decrease
pad=320:180:(ow-iw)/2:(oh-ih)/2
```

避免：

```text
拉伸
```

---

# 四十六、建议新增模块

新增：

```text
src-tauri/src/learning/thumbnail.rs
```

负责：

```text
Thumbnail Resolver
Remote Cache
FFmpeg抽帧
Cache Key
Cache Invalid
Fallback
Cleanup
```

---

# 四十七、Thumbnail Cache Key

本地文件：

```text
normalized path
+
size
+
mtime
```

网络视频：

优先：

```text
source_video_id
```

没有则：

```text
source_url hash
```

---

# 四十八、缓存失效

如果：

```text
路径相同
```

但：

```text
size
mtime
```

变化：

旧 generated thumbnail：

```text
失效
```

重新生成。

---

# 四十九、Thumbnail 生成时机

必须覆盖：

```text
本地导入完成
Playlist导入
下载完成
Series更新
文件重新定位
用户手动重新生成
```

---

# 五十、Thumbnail 后台任务

Thumbnail 生成：

```text
异步
```

不要让：

```text
导入300节教程
```

必须等：

```text
300张图全部生成
```

才能进入 Series。

流程：

```text
先导入
↓
UI可用
↓
后台逐步补图
```

---

# 五十一、Thumbnail FFmpeg 并发

建议：

```text
最多1~2个
```

同时运行。

避免：

```text
500视频
↓
500个FFmpeg
↓
电脑卡死
```

---

# 五十二、Thumbnail 任务去重

同一：

```text
item_id
```

同时只允许：

```text
一个ThumbnailJob
```

---

# 五十三、课程列表 UI 组件

建议新增：

```text
src/features/learning/LessonRow.tsx
src/features/learning/LessonThumbnail.tsx
```

将课程列表逻辑从：

```text
LearningLibraryPage.tsx
```

拆出来。

---

# 五十四、Lesson Thumbnail 内容

包含：

```text
缩略图
时长角标
进度条
完成Badge
下载状态
缺失状态
Playing状态
```

---

# 五十五、课程行布局

推荐：

```text
┌──────────────┐
│ Thumbnail    │  课程标题
│         24:11│  已观看 12:32 / 24:11 · 52%
│██████░░░░░░░│  进行中
└──────────────┘                    [Actions]
```

---

# 五十六、标题规则

课程标题：

```text
最多2行
```

超过：

```text
ellipsis
```

---

# 五十七、时长 Badge

Thumbnail：

```text
右下角
```

显示：

```text
24:11
```

使用：

```text
半透明深色背景
```

---

# 五十八、观看进度

如果：

```text
position > 0
completed = false
```

Thumbnail 底部显示：

```text
Progress Bar
```

---

# 五十九、完成状态

如果：

```text
completed = true
```

显示：

```text
✓
```

小 Badge。

不要整张图片覆盖绿色。

---

# 六十、当前播放高亮

如果：

```text
playingItem === item.id
```

课程行：

```text
高亮
```

同时显示：

```text
正在播放
```

---

# 六十一、未下载课程

即使：

```text
available = false
```

如果：

```text
source_thumbnail_url
```

存在：

仍显示缩略图。

覆盖：

```text
等待下载
下载中
下载失败
```

---

# 六十二、文件缺失

如果：

```text
file missing
```

Thumbnail：

```text
降低亮度
```

覆盖：

```text
文件缺失
```

继续允许：

```text
重新定位
```

---

# 六十三、Lazy Load

Thumbnail：

```html
loading="lazy"
```

必须开启。

---

# 六十四、图片加载失败

Fallback：

```text
Item Thumbnail
↓
Series Cover
↓
Placeholder
```

禁止显示 Broken Image。

---

# 六十五、手动更换缩略图

P1。

课程菜单：

```text
更换缩略图
```

支持：

```text
JPEG
PNG
WebP
```

手动图片：

```text
thumbnail_source = manual
```

优先级最高。

---

# 六十六、重新生成 Thumbnail

菜单：

```text
重新生成缩略图
```

generated：

```text
直接重新生成
```

manual：

```text
先确认
```

禁止静默覆盖用户自定义图。

---

# 六十七、Thumbnail Cache 设置

学习设置增加：

```text
缩略图缓存
```

显示：

```text
当前大小：324 MB
```

按钮：

```text
清理未使用缩略图
```

---

# 六十八、缓存清理规则

只删除：

```text
数据库中不再引用
```

的缓存。

禁止：

```text
直接清空整个目录
```

作为普通清理。

---

# 六十九、课程删除

删除 Item：

如果是：

```text
generated
remote cached
```

且无其他引用：

允许清理。

如果：

```text
manual
```

不要删除用户原图片。

---

# 七十、第三部分
# 两套系统协同

---

# 七十一、多格式 Playback 与 Thumbnail 协同

例如：

```text
lesson01.wmv
```

Thumbnail：

```text
直接从WMV原文件抽帧
```

Playback：

```text
WMV
↓
Transcode
↓
Proxy MP4
```

不要默认从：

```text
Proxy
```

生成 Thumbnail。

---

# 七十二、Proxy 被清理

即使：

```text
playback proxy被删除
```

仍然保留：

```text
Thumbnail
PlaybackProgress
Notes
```

---

# 七十三、Thumbnail 被清理

即使：

```text
thumbnail被删除
```

仍然：

```text
视频能播放
```

后台可重新生成。

---

# 七十四、课程重新定位

视频重新定位后：

```text
Playback Cache
Thumbnail Cache
```

都检查：

```text
是否仍然匹配原文件
```

如果不匹配：

```text
分别失效
```

但：

```text
Progress
Notes
Completed
```

保持。

---

# 七十五、第四部分
# 设置

---

# 七十六、播放兼容设置

学习库设置增加：

```text
播放兼容性
```

模式：

```text
● 自动
○ 优先原文件
○ 始终生成兼容版本
```

默认：

```text
自动
```

---

# 七十七、硬件加速

不要增加第二套 GPU 设置。

直接复用：

```text
Media Pro GPU Settings
```

如果需要显示：

```text
硬件加速：自动
```

只引用现有设置。

---

# 七十八、播放缓存设置

显示：

```text
兼容播放缓存：
18.6 GB
```

提供：

```text
清理未使用代理
```

可设置：

```text
最大缓存
自动清理周期
```

---

# 七十九、缩略图缓存设置

显示：

```text
缩略图缓存：
324 MB
```

提供：

```text
清理未使用缩略图
```

---

# 八十、第五部分
# 数据库迁移

---

# 八十一、Migration 原则

必须保留：

```text
Categories
Series
Items
Progress
Notes
Favorite
SortOrder
DownloadTask Binding
Source Metadata
```

---

# 八十二、数据库升级失败

如果：

```text
learning.db migration失败
```

必须保持现有隔离：

```text
Downloader仍可使用
Media Pro仍可使用
```

学习库显示错误即可。

---

# 八十三、第六部分
# 性能

---

# 八十四、大库要求

继续测试：

```text
100 Categories
1000 Series
10000 Items
```

---

# 八十五、大Series

测试：

```text
500课
```

继续：

```text
每页50
```

---

# 八十六、Thumbnail 并发

最大：

```text
1~2 FFmpeg
```

---

# 八十七、Proxy 并发

播放器 Proxy：

优先级高于：

```text
后台Thumbnail
```

用户点击课程后：

如果需要转码：

```text
Playback Proxy Task
```

应优先执行。

不要因为：

```text
后台正在生成200张缩略图
```

让用户等很久。

---

# 八十八、任务优先级建议

```text
1. 用户当前播放Proxy
2. 用户主动重新生成Thumbnail
3. Playlist Remote Thumbnail
4. Background Generated Thumbnail
```

如果现有任务系统不支持 Priority：

至少保证播放器 Proxy 不被后台Thumbnail阻塞。

---

# 八十九、第七部分
# 前端建议结构

建议：

```text
src/features/learning/

LearningLibraryPage.tsx
LearningPlayer.tsx

LessonRow.tsx
LessonThumbnail.tsx

PlaybackCompatibilityStatus.tsx
```

无需为了此次版本引入：

```text
Redux
Zustand
```

---

# 九十、第八部分
# 后端建议结构

建议：

```text
src-tauri/src/learning/

compatibility.rs
thumbnail.rs
```

保留：

```text
player.rs
repository.rs
service.rs
```

---

# 九十一、第九部分
# 测试素材

至少准备：

```text
MP4 H264 AAC
MKV H264 AAC
MKV HEVC AAC
MOV H264
AVI MPEG4
WMV WMV3 WMA
WMV VC1 WMA
MPEG MPEG2
TS H264
M2TS H264
WebM VP9 Opus
FLV
```

同时：

```text
16:9
4:3
9:16
1:1
```

---

# 九十二、多格式播放测试

每一种格式验证：

```text
Import
ffprobe
Playback Plan
Direct / Remux / Transcode
Playback
Seek
Rate
Pause
Resume
Progress Save
Restart
Continue Learning
```

---

# 九十三、WMV完整验收

必须真实验证：

```text
导入 .wmv
↓
课程列表出现Thumbnail
↓
点击播放
↓
自动识别Transcode
↓
生成H264/AAC Proxy
↓
自动播放
↓
Seek
↓
1.5x
↓
退出应用
↓
重新打开
↓
恢复位置
```

---

# 九十四、MKV测试

至少测试：

```text
MKV H264 AAC
```

应优先：

```text
Remux
```

不要无意义重编码。

---

# 九十五、HEVC测试

测试：

```text
MKV/MP4 HEVC
```

根据当前 WebView2 / OS 能力：

```text
Direct
或
Transcode
```

不要盲目固定策略。

---

# 九十六、Thumbnail测试

至少：

```text
MP4
MKV
MOV
WMV
```

生成 Thumbnail。

---

# 九十七、Thumbnail比例

验证：

```text
4:3
9:16
1:1
```

不拉伸。

---

# 九十八、Remote Thumbnail测试

Playlist：

```text
thumbnail URL
↓
缓存
↓
断网
↓
重启
↓
仍显示
```

---

# 九十九、观看进度UI测试

例如：

```text
duration = 20min
position = 10min
```

显示：

```text
约50% Progress Bar
```

---

# 一百、完成状态测试

```text
completed=true
```

显示：

```text
Complete Badge
```

---

# 一百零一、Playing状态测试

播放：

```text
Lesson 4
```

仅：

```text
Lesson 4
```

高亮。

---

# 一百零二、Missing测试

移动文件：

```text
课程显示文件缺失
Thumbnail仍保留
重新定位可用
```

---

# 一百零三、Proxy Cache测试

第一次 WMV：

```text
生成Proxy
```

第二次：

```text
直接命中缓存
```

禁止再次完整转码。

---

# 一百零四、Thumbnail Cache测试

第一次：

```text
抽帧
```

第二次：

```text
直接缓存
```

---

# 一百零五、Cache Invalid测试

原文件：

```text
size变化
mtime变化
```

Playback Proxy：

```text
失效
```

Thumbnail：

```text
失效
```

然后分别重建。

---

# 一百零六、原文件保护

至少：

```text
WMV
MKV
MP4
```

执行：

```text
Before SHA256
After SHA256
```

完全一致。

---

# 一百零七、第十部分
# Windows 实机验收

---

# 一百零八、至少完成以下实机测试

## Test 1

导入 MP4。

```text
Direct Playback
Thumbnail
```

---

## Test 2

导入 MKV H264 AAC。

```text
Remux
Thumbnail
```

---

## Test 3

导入 WMV。

```text
Transcode Proxy
Thumbnail
```

---

## Test 4

导入 AVI。

```text
自动兼容播放
```

---

## Test 5

导入 HEVC。

```text
自动选择Direct或Transcode
```

---

## Test 6

WMV观看到50%。

关闭。

重开。

```text
恢复位置
```

---

## Test 7

Thumbnail显示：

```text
Duration
Progress
Completed
Playing
```

---

## Test 8

导入 Playlist。

未下载课程：

```text
显示源站Thumbnail
```

---

## Test 9

移动视频。

```text
Missing
↓
Relink
↓
重新播放
```

---

## Test 10

关闭应用重开。

```text
Proxy Cache命中
Thumbnail Cache命中
```

---

# 一百零九、第十一部分
# UI尺寸

测试：

```text
1600
1440
1200
1000
880
```

要求：

```text
无横向溢出
Thumbnail不挤压标题
操作按钮可用
播放器正常
```

---

# 一百一十、高 DPI

至少验证：

```text
125%
150%
```

---

# 一百一十一、第十二部分
# 自动测试

运行：

```text
npm test
```

以及：

```text
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

---

# 一百一十二、Rust新增测试

至少覆盖：

```text
Playback Compatibility Planner

Direct判断

Remux判断

Transcode判断

WMV检测

Proxy Cache Key

Proxy Cache Hit

Proxy Cache Invalid

GPU fallback

Thumbnail Cache Key

Thumbnail Cache Hit

Thumbnail Cache Invalid

WMV Thumbnail

Remote Thumbnail fallback

Aspect Ratio

Original File Protection
```

---

# 一百一十三、前端新增测试

至少：

```text
Thumbnail fallback

Duration badge

Progress bar

Completed badge

Playing highlight

Missing state

Preparing playback state

Proxy progress state

Unsupported state
```

---

# 一百一十四、第十三部分
# 回归测试

必须重新验证：

```text
Download
Playlist解析
Cookie
Proxy
暂停
恢复
取消
历史
Media Pro
Trim
Compress
Subtitle
Batch
学习进度
笔记
自动下一课
系列更新
```

---

# 一百一十五、第十四部分
# 版本号

统一：

```text
0.4.1
```

更新：

```text
package.json
Cargo.toml
tauri.conf.json
UI版本
```

---

# 一百一十六、README

更新：

```text
多格式学习视频播放
自动Remux
自动兼容转码
WMV支持
MKV支持
AVI支持
课程缩略图
源站缩略图缓存
本地自动抽帧
观看进度缩略图
```

---

# 一百一十七、文档

新增或更新：

```text
docs/learning-playback-v0.4.1.md
docs/learning-playback-v0.4.1-delivery.md
```

---

# 一百一十八、第十五部分
# Git 开发要求

当前基础优先：

```text
codex/learning-library-v0.4
```

如果已经有：

```text
codex/learning-multiformat-v0.4.1
```

或：

```text
codex/learning-thumbnail-v0.4.1
```

禁止继续制造第三条冲突分支。

先检查：

```text
哪个包含最新功能
```

然后统一开发到：

```text
codex/learning-playback-thumbnail-v0.4.1
```

或者在实际最新 v0.4.1 分支上继续。

---

# 一百一十九、PR要求

创建：

```text
Draft PR
```

不要：

```text
自动Merge
发布Release
覆盖旧PR历史
```

---

# 一百二十、Codex最终交付报告

必须输出：

```text
1 修改文件完整列表

2 数据库Migration

3 Playback Compatibility Planner设计

4 Direct播放规则

5 Remux规则

6 Transcode Proxy规则

7 GPU fallback

8 Proxy Cache结构

9 Proxy Cache失效策略

10 Thumbnail Resolver设计

11 Remote Thumbnail Cache

12 FFmpeg抽帧规则

13 Thumbnail Cache策略

14 Learning Player UI变化

15 Lesson List UI变化

16 自动测试结果

17 Windows实机结果

18 WMV结果

19 MKV结果

20 HEVC结果

21 大库性能结果

22 原文件SHA256保护结果

23 NSIS构建结果

24 已知限制

25 下一阶段建议
```

---

# 一百二十一、本版本禁止扩大范围

不要在本次顺手加入：

```text
Whisper
AI摘要
字幕全文搜索
AI章节
语义搜索
Quiz
Flashcards
DCC Bridge
云同步
账号系统
多用户
完整学习统计
Hover动态预览
HLS服务器
完整NLE
```

---

# 一百二十二、最终产品体验

用户最终不需要关心：

```text
这个文件到底是MP4还是WMV
这个视频是VC1还是H264
这个MKV浏览器能不能播
```

用户应该只需要：

```text
点击课程
```

映流自动：

```text
ffprobe
↓
Direct?
↓
Remux?
↓
Transcode?
↓
播放
```

同时课程列表：

```text
自动显示缩略图
自动显示时长
自动显示观看进度
自动显示完成状态
自动高亮当前视频
```

最终实现：

```text
只要FFmpeg能够正常解码的视频
↓
映流都尽可能让它能够播放
```

---

# 一百二十三、最终验收链路

完整流程：

```text
打开映流

↓

进入学习库

↓

导入包含：

MP4
MKV
WMV
AVI
MOV

的课程目录

↓

所有课程进入Series

↓

所有课程逐步生成Thumbnail

↓

用户点击MP4

↓

Direct播放

↓

用户点击MKV

↓

自动Remux

↓

用户点击WMV

↓

自动生成H264/AAC兼容Proxy

↓

自动播放

↓

播放到一半

↓

课程Thumbnail显示进度

↓

关闭映流

↓

重新启动

↓

继续学习

↓

恢复原播放位置

↓

Proxy直接命中缓存

↓

Thumbnail直接命中缓存
```

整个过程中：

```text
原视频不被修改
学习进度不丢失
笔记不丢失
缩略图不重复生成
Proxy不重复转码
Downloader不受影响
Media Pro不受影响
```

---

# 一百二十四、最终执行要求

不要只输出：

```text
方案
TODO
Mock
占位UI
```

必须直接完成：

```text
多格式导入
ffprobe兼容检测
Direct Playback
Remux
Transcode Proxy
WMV播放
MKV播放
AVI播放
HEVC兼容处理
GPU复用
Proxy Cache
字幕兼容
课程缩略图
Remote Thumbnail
FFmpeg抽帧
Thumbnail Cache
课程列表UI
进度显示
状态显示
数据库Migration
自动测试
Windows实机测试
性能测试
NSIS构建
Draft PR
最终交付报告
```

优先遵循：

```text
当前项目真实架构
```

如果现有代码已经有更好的通用实现：

```text
直接复用
```

不要为了机械满足任务书而复制逻辑。

开始执行。