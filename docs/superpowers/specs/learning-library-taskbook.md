# 映流 v0.4.0
# Learning Library
# 学习库 / 系列教程播放系统
# Codex 执行文档

---

# 一、项目背景

项目：

映流（video_downloader）

当前版本：

v0.3.0

目标版本：

v0.4.0

版本名称：

Learning Library

中文名称：

学习库

---

# 二、本版本核心目标

映流目前已经具备：

- yt-dlp 视频下载
- 播放列表解析
- 视频 / 音频 / 字幕下载
- 下载队列
- 历史记录
- FFmpeg Media Pro
- 视频裁剪
- 视频压缩
- GPU 编码检测
- 字幕处理
- 批量处理
- 抽帧
- 联系表
- 封面
- Metadata

v0.4 不继续堆 FFmpeg 工具。

本版本新增一个完整的：

# Learning Library

核心使用场景：

用户下载：

- UE5 系列教程
- Blender 系列教程
- C4D 系列教程
- AI 视频教程
- 编程课程
- 语言学习视频
- 其他系列教学内容

下载完成后可以：

```text
下载
↓
加入学习库
↓
按分类整理
↓
按系列观看
↓
自动记录学习进度
↓
继续学习
↓
记录时间点笔记
```

最终让映流从：

```text
视频下载器 + 媒体工具
```

升级为：

```text
下载
+
本地视频库
+
系列教程管理
+
学习播放器
+
学习进度管理
```

---

# 三、重要开发原则

## 1. 不破坏 v0.3

必须保持以下功能正常：

- 下载
- 播放列表解析
- Cookie
- Proxy
- yt-dlp 更新
- 下载暂停 / 恢复 / 取消
- 下载历史
- Media Pro
- FFmpeg
- 裁剪
- 压缩
- 字幕
- 批处理
- 托盘运行

---

## 2. 不重写现有 DownloadTask

现有下载任务继续使用原来的任务模型。

不要为了学习库重构全部下载系统。

---

## 3. 不迁移现有 DiskState

现有：

```text
settings
tasks
cookie
proxy
```

继续保持当前持久化方式。

学习库使用独立 SQLite：

```text
learning.db
```

不要为了 v0.4 将整个项目迁移 SQLite。

---

## 4. 下载系统与学习库解耦

Downloader 负责：

```text
获取文件
```

Learning Library 负责：

```text
管理教程
```

两者通过：

```text
下载完成输出
```

连接。

---

# 四、主导航升级

当前：

```text
下载
媒体工具
历史
设置
```

修改为：

```text
下载

学习库

媒体工具

历史

设置
```

学习库使用独立页面。

---

# 五、学习库核心数据结构

采用三级结构：

```text
Category
   ↓
Series
   ↓
LearningItem
```

对应：

```text
分类
   ↓
教程系列
   ↓
课程视频
```

例如：

```text
UE5

├ Niagara 完整教程
│
├ 材质系统教程
│
└ Sequencer 教程
```

进入：

```text
Niagara 完整教程

01 基础介绍
02 Emitter
03 Particle System
04 GPU Simulation
05 Collision
...
```

---

# 六、数据库

新增：

```text
learning.db
```

推荐位置：

```text
应用数据目录/
learning.db
```

不要放在下载目录。

---

# 七、新增 Rust 模块

新增：

```text
src-tauri/src/learning/
```

结构：

```text
learning/

├ mod.rs
├ db.rs
├ models.rs
├ repository.rs
├ service.rs
├ category.rs
├ series.rs
├ item.rs
├ playback.rs
├ notes.rs
├ importer.rs
└ updater.rs
```

如果当前项目已有统一数据库辅助模块，可合理复用。

不要为了完全匹配本任务书而制造不必要的文件。

---

# 八、数据库初始化

应用启动时：

```text
检查 learning.db
↓
数据库不存在
↓
创建
↓
执行 schema
```

数据库升级必须有：

```text
schema_version
```

不要依赖删除数据库重新创建。

未来必须支持 migration。

---

# 九、categories 表

创建：

```sql
CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    icon TEXT,
    color TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);
```

用途：

学习内容分类。

例如：

```text
UE5

AI 视频

Blender

C4D

编程

韩语
```

---

# 十、series 表

创建：

```sql
CREATE TABLE series (
    id TEXT PRIMARY KEY,
    category_id TEXT,
    title TEXT NOT NULL,
    description TEXT,
    cover_path TEXT,
    source_type TEXT,
    source_url TEXT,
    source_playlist_id TEXT,
    author TEXT,
    total_items INTEGER NOT NULL DEFAULT 0,
    completed_items INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY(category_id) REFERENCES categories(id)
);
```

---

# 十一、series_items 表

创建：

```sql
CREATE TABLE series_items (
    id TEXT PRIMARY KEY,
    series_id TEXT NOT NULL,

    title TEXT NOT NULL,

    file_path TEXT,

    source_url TEXT,

    source_video_id TEXT,

    source_playlist_index INTEGER,

    duration_ms INTEGER,

    sort_order INTEGER NOT NULL DEFAULT 0,

    thumbnail_path TEXT,

    download_task_id TEXT,

    available INTEGER NOT NULL DEFAULT 1,

    created_at INTEGER NOT NULL,

    updated_at INTEGER NOT NULL,

    FOREIGN KEY(series_id) REFERENCES series(id)
);
```

---

# 十二、playback_progress 表

创建：

```sql
CREATE TABLE playback_progress (
    item_id TEXT PRIMARY KEY,

    position_ms INTEGER NOT NULL DEFAULT 0,

    duration_ms INTEGER NOT NULL DEFAULT 0,

    completed INTEGER NOT NULL DEFAULT 0,

    playback_rate REAL NOT NULL DEFAULT 1.0,

    last_played_at INTEGER,

    updated_at INTEGER NOT NULL,

    FOREIGN KEY(item_id) REFERENCES series_items(id)
);
```

---

# 十三、video_notes 表

创建：

```sql
CREATE TABLE video_notes (
    id TEXT PRIMARY KEY,

    item_id TEXT NOT NULL,

    time_ms INTEGER NOT NULL,

    content TEXT NOT NULL,

    created_at INTEGER NOT NULL,

    updated_at INTEGER NOT NULL,

    FOREIGN KEY(item_id) REFERENCES series_items(id)
);
```

---

# 十四、可选 indexes

至少增加：

```sql
CREATE INDEX idx_series_category
ON series(category_id);

CREATE INDEX idx_items_series
ON series_items(series_id);

CREATE INDEX idx_items_source_video
ON series_items(source_video_id);

CREATE INDEX idx_items_playlist_index
ON series_items(series_id, source_playlist_index);

CREATE INDEX idx_progress_last_played
ON playback_progress(last_played_at);

CREATE INDEX idx_notes_item
ON video_notes(item_id);
```

---

# 十五、Rust Models

新增：

## LearningCategory

```rust
pub struct LearningCategory {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
}
```

---

## LearningSeries

```rust
pub struct LearningSeries {
    pub id: String,
    pub category_id: Option<String>,

    pub title: String,
    pub description: Option<String>,

    pub cover_path: Option<String>,

    pub source_type: Option<String>,
    pub source_url: Option<String>,
    pub source_playlist_id: Option<String>,

    pub author: Option<String>,

    pub total_items: i64,
    pub completed_items: i64,
}
```

---

## LearningItem

```rust
pub struct LearningItem {
    pub id: String,
    pub series_id: String,

    pub title: String,

    pub file_path: Option<String>,

    pub source_url: Option<String>,
    pub source_video_id: Option<String>,

    pub source_playlist_index: Option<i64>,

    pub duration_ms: Option<i64>,

    pub sort_order: i64,

    pub thumbnail_path: Option<String>,

    pub download_task_id: Option<String>,

    pub available: bool,
}
```

---

## PlaybackProgress

```rust
pub struct PlaybackProgress {
    pub item_id: String,

    pub position_ms: i64,

    pub duration_ms: i64,

    pub completed: bool,

    pub playback_rate: f64,

    pub last_played_at: Option<i64>,
}
```

---

## VideoNote

```rust
pub struct VideoNote {
    pub id: String,

    pub item_id: String,

    pub time_ms: i64,

    pub content: String,

    pub created_at: i64,

    pub updated_at: i64,
}
```

---

# 十六、分类管理

学习库必须支持：

```text
创建分类

重命名分类

修改分类

删除分类

拖动排序
```

删除分类时：

不要删除视频文件。

如果分类包含 Series：

必须：

```text
提示用户
```

允许：

```text
移动其中系列到未分类
```

或者：

```text
取消删除
```

禁止静默删除整个学习库内容。

---

# 十七、默认分类

首次启动学习库时：

不要创建大量默认分类。

可以仅创建：

```text
未分类
```

其他全部由用户自行创建。

---

# 十八、Series 管理

用户可以：

```text
新建系列
```

填写：

```text
系列名称

分类

封面

描述

来源链接
```

---

Series 支持：

```text
编辑

删除

移动分类

修改封面

修改排序

打开来源网页
```

---

删除 Series 时：

必须提供两个明确选项：

```text
仅从学习库移除

从学习库移除并删除本地文件
```

默认：

```text
仅从学习库移除
```

禁止默认删除文件。

---

# 十九、学习系列封面

封面来源支持：

```text
自动使用第一课缩略图

从源站 Playlist 获取

手动选择本地图片
```

Series 没有封面时：

使用统一默认课程封面。

---

# 二十、手动添加本地教程

必须支持：

```text
+ 添加视频
```

支持：

```text
单个文件

多个文件

整个文件夹

拖拽
```

---

支持常见格式：

```text
mp4
mkv
mov
webm
avi
m4v
wmv
ts
```

---

# 二十一、文件夹导入

例如目录：

```text
UE5 Niagara教程/

01 Niagara介绍.mp4
02 Emitter.mp4
03 Particle.mp4
04 Collision.mp4
```

导入后：

自动排序：

```text
01
02
03
04
```

优先识别：

```text
数字前缀
```

例如：

```text
1
01
001
第1课
第01课
Lesson 01
EP01
```

如果无法识别：

按：

```text
自然文件名排序
```

不要使用简单字符串排序造成：

```text
1
10
11
2
3
```

---

# 二十二、ffprobe 集成

本地视频添加到学习库后：

使用现有 ffprobe 能力读取：

```text
duration
width
height
codec
fps
```

学习库第一阶段最少保存：

```text
duration_ms
```

不要再实现另一套 ffprobe。

复用 v0.3 现有媒体探测能力。

---

# 二十三、播放器

新增专用：

# Learning Player

不要直接复用 Media Pro 裁剪编辑器。

可以复用其底层安全媒体加载能力。

播放器目标是：

```text
观看教程
```

不是：

```text
编辑视频
```

---

# 二十四、播放器界面

Series 页面采用：

```text
┌─────────────────────────────────────┐
│                                     │
│             视频播放器               │
│                                     │
└─────────────────────────────────────┘

播放 / 暂停

进度条

当前时间 / 总时间

-10 秒

+10 秒

倍速

字幕

音量

全屏

上一课

下一课
```

---

# 二十五、播放速度

支持：

```text
0.5x

0.75x

1.0x

1.25x

1.5x

1.75x

2.0x
```

播放速度：

保存到：

```text
playback_progress.playback_rate
```

重新打开该视频：

恢复。

---

# 二十六、继续播放

播放过程中：

定期保存：

```text
position_ms
```

推荐：

每：

```text
5 秒
```

保存一次。

同时在：

```text
暂停

切换视频

关闭页面

应用退出
```

时立即保存。

不要每个 timeupdate 都写 SQLite。

---

# 二十七、自动完成

当：

```text
position / duration >= 90%
```

自动：

```text
completed = true
```

UI显示：

```text
✓ 已完成
```

---

用户也可以：

```text
手动标记完成
```

以及：

```text
标记为未完成
```

---

# 二十八、继续学习

学习库首页增加：

# 继续学习

排序：

```text
last_played_at DESC
```

只显示：

```text
未完成课程
```

卡片显示：

```text
系列封面

系列名称

当前课时

学习进度

上次观看时间
```

按钮：

```text
继续学习
```

点击：

恢复上次：

```text
视频

时间点

播放速度
```

---

# 二十九、自动播放下一课

当前课播放结束后：

显示：

```text
本课已完成

5秒后播放下一课

下一课：
05 GPU Simulation

[立即播放]

[取消]
```

支持设置：

```text
自动播放下一课
```

默认：

```text
开启
```

---

# 三十、课程列表

Series 页面右侧或下方显示：

```text
✓ 01 基础介绍       08:24

✓ 02 Emitter        15:42

▶ 03 Particle       22:10

○ 04 Collision      18:55
```

状态：

```text
✓ 已完成

▶ 当前

○ 未学习
```

---

课程列表支持：

```text
点击播放

拖拽排序

手动完成

重命名

从系列移除

打开文件位置
```

---

# 三十一、Series 总进度

显示：

```text
15 / 32 已完成
```

以及：

```text
47%
```

进度由：

```text
completed_items / total_items
```

计算。

避免依赖可能不同步的缓存值。

如果保留缓存字段：

每次修改 Item/Progress 时必须同步。

---

# 三十二、时间点笔记

播放教程时：

新增：

```text
添加笔记
```

点击后：

自动记录当前：

```text
time_ms
```

用户输入：

```text
这里讲 Niagara GPU Emitter 设置
```

保存：

```text
12:35
这里讲 Niagara GPU Emitter 设置
```

---

# 三十三、笔记列表

播放器页面显示：

```text
课程笔记

03:22 这一段解释了Emitter生命周期

12:35 GPU Emitter参数

18:04 Collision注意事项
```

点击：

```text
12:35
```

播放器直接跳转。

---

笔记支持：

```text
新建

编辑

删除
```

---

# 三十四、下载页面与学习库融合

这是本版本核心之一。

现有代码已经有：

```text
MediaPreview.isPlaylist

PlaylistEntry[]

playlistIndex
```

必须继续复用。

不要重新做第二套 Playlist 解析器。

---

# 三十五、Playlist 下载新增选项

当解析到：

```text
isPlaylist = true
```

在下载设置中增加：

# 学习库

```text
☐ 下载后加入学习库
```

开启后显示：

```text
系列名称

分类

☑ 创建新的系列

☑ 保持源播放列表顺序
```

---

# 三十六、创建学习系列

用户解析一个在线播放列表：

例如：

```text
UE5 Niagara完整教程
32个视频
```

选择：

```text
下载后加入学习库
```

创建：

```text
Series
```

Series：

```text
title = Playlist title

source_url = Playlist URL

source_type = 当前解析平台
```

每一个 PlaylistEntry：

创建对应：

```text
LearningItem
```

---

# 三十七、重要：下载尚未完成时

学习库 Item 可以先存在。

但是：

```text
file_path = null

available = false
```

UI显示：

```text
等待下载
```

下载完成后：

```text
file_path = 实际文件

available = true
```

---

# 三十八、下载失败

如果下载失败：

学习 Item 保留。

状态显示：

```text
下载失败
```

允许：

```text
重新下载
```

不要删除 Series Item。

---

# 三十九、DownloadTask 与 LearningItem 关联

LearningItem 保存：

```text
download_task_id
```

用于跟踪：

```text
哪个任务生成了哪个学习视频
```

---

不要修改 DownloadTask 的核心生命周期。

只增加必要的关联接口。

---

# 四十、源站视频 ID

创建 LearningItem 时必须尽量保存：

```text
source_video_id
```

用于：

```text
去重

Playlist 更新检查
```

不要只保存文件路径。

---

# 四十一、播放列表顺序

在线播放列表导入：

优先使用当前已有：

```text
playlistIndex
```

写入：

```text
source_playlist_index
sort_order
```

不要根据下载完成顺序排序。

---

# 四十二、Series 更新检测

P1 功能。

对于存在：

```text
source_url
```

的 Series：

提供：

```text
检查更新
```

流程：

```text
Series source_url

↓

yt-dlp重新解析Playlist

↓

比较source_video_id

↓

发现新增视频

↓

显示更新
```

例如：

```text
已有 32 课

发现 2 个新增视频

33 Niagara Advanced
34 Optimization
```

用户选择：

```text
下载新增课程
```

---

# 四十三、更新检测禁止行为

不要：

```text
因为标题改变就创建重复课程
```

优先：

```text
source_video_id
```

识别。

如果无法获取 ID：

才考虑：

```text
source_url
```

---

# 四十四、学习库搜索

支持搜索：

```text
分类名称

系列名称

课程名称

笔记内容
```

第一版使用 SQLite 普通搜索即可。

如果当前规模小：

不要为了 v0.4 引入 Elasticsearch。

不要引入外部服务器。

---

# 四十五、收藏

P1。

Series 支持：

```text
收藏
```

增加字段：

```text
favorite
```

学习库首页增加：

```text
收藏系列
```

---

# 四十六、学习库首页

页面结构：

```text
学习库


继续学习
────────────────────────

[课程卡片] [课程卡片] [课程卡片]


分类
────────────────────────

UE5

AI视频

Blender

C4D


最近添加
────────────────────────

[系列卡片]

[系列卡片]

[系列卡片]
```

---

# 四十七、Series Card

卡片显示：

```text
封面

系列名称

分类

课时数量

总时长

完成进度
```

例如：

```text
UE5 Niagara完整教程

32课 · 14小时

████████░░ 72%
```

---

# 四十八、Learning Library 前端结构

建议新增：

```text
src/features/learning/
```

结构：

```text
learning/

├ LearningLibraryPage.tsx

├ LearningSidebar.tsx

├ ContinueLearning.tsx

├ CategoryList.tsx

├ CategoryEditor.tsx

├ SeriesGrid.tsx

├ SeriesCard.tsx

├ SeriesPage.tsx

├ SeriesHeader.tsx

├ SeriesItemList.tsx

├ LearningPlayer.tsx

├ PlayerControls.tsx

├ PlaybackRateMenu.tsx

├ VideoNotes.tsx

├ NoteEditor.tsx

├ ImportLocalMedia.tsx

├ PlaylistImportOptions.tsx

└ SeriesUpdateDialog.tsx
```

可以根据当前前端结构适当调整。

不要强行制造几十个极小组件。

---

# 四十九、TypeScript Types

新增：

```text
src/types/learning.ts
```

至少包含：

```text
LearningCategory

LearningSeries

LearningItem

PlaybackProgress

VideoNote

SeriesDetail

ContinueLearningItem
```

---

# 五十、Tauri Commands

建议至少提供：

```text
learning_list_categories

learning_create_category

learning_update_category

learning_delete_category


learning_list_series

learning_get_series

learning_create_series

learning_update_series

learning_delete_series


learning_add_items

learning_remove_item

learning_update_item

learning_reorder_items


learning_get_progress

learning_save_progress

learning_mark_completed


learning_list_notes

learning_create_note

learning_update_note

learning_delete_note


learning_import_files

learning_import_directory


learning_create_series_from_playlist

learning_attach_download_output


learning_check_series_updates
```

具体命名可以遵循项目现有 Tauri command 规范。

---

# 五十一、媒体文件安全

学习库播放本地媒体必须沿用当前 Tauri 安全模型。

不要允许前端任意读取：

```text
C:\
```

播放器只授权实际学习库媒体文件。

---

# 五十二、文件移动问题

必须考虑：

用户移动或删除本地视频。

打开 LearningItem 时：

如果：

```text
file_path 不存在
```

不要崩溃。

显示：

```text
文件已移动或不存在
```

提供：

```text
重新定位文件
```

按钮。

重新选择文件后：

更新：

```text
file_path
available
duration
```

---

# 五十三、文件删除

Series 删除时：

不要自动删除文件。

LearningItem 删除：

默认：

```text
仅从学习库移除
```

如果用户主动选择：

```text
同时删除本地文件
```

必须二次确认。

---

# 五十四、下载文件不能被学习库随意移动

第一版：

学习库使用：

```text
引用文件路径
```

不要自动把所有教程搬进自己的媒体目录。

避免：

- 重复占空间
- 破坏下载历史
- 打断下载续传
- 破坏 Media Pro 引用

未来可以再增加：

```text
整理到学习库目录
```

但不属于 v0.4 P0。

---

# 五十五、学习统计 P2

本版本只预留。

未来支持：

```text
今日学习时间

本周学习时间

完成课时

学习天数

不同分类学习时间
```

v0.4 不强制完整实现。

---

# 五十六、字幕

Learning Player：

如果视频包含字幕轨：

允许选择。

如果旁边存在：

```text
video.srt

video.ass

video.vtt
```

可以尝试加载。

尽量复用 v0.3 字幕能力。

不要在 v0.4 实现复杂字幕识别。

---

# 五十七、播放器快捷键

建议：

```text
Space

播放 / 暂停


←

后退5秒


→

前进5秒


Shift + ←

后退10秒


Shift + →

前进10秒


M

静音


F

全屏


N

下一课


P

上一课
```

输入笔记时：

不要拦截文字输入快捷键。

---

# 五十八、Series 导入体验

支持直接拖：

```text
一个文件夹
```

到学习库。

流程：

```text
拖入文件夹

↓

扫描视频

↓

显示预览

↓

建议系列名称 = 文件夹名称

↓

用户选择分类

↓

确认导入
```

---

# 五十九、导入预览

导入前显示：

```text
检测到 32 个视频

01 xxxx.mp4

02 xxxx.mp4

03 xxxx.mp4

...

系列名称：
[UE5 Niagara完整教程]

分类：
[UE5]
```

用户确认后才写数据库。

---

# 六十、重复导入

必须避免明显重复。

本地文件第一阶段至少按：

```text
规范化绝对路径
```

检测。

网络课程：

优先：

```text
source_video_id
```

检测。

如果重复：

显示：

```text
已有 30 个

新增 2 个

重复 30 个
```

不要再次创建全部内容。

---

# 六十一、学习库设置

设置新增：

# 学习库

选项：

```text
自动播放下一课

完成阈值

默认播放速度

继续播放时恢复位置

自动加载同名字幕
```

默认：

```text
自动播放下一课 = true

完成阈值 = 90%

默认播放速度 = 1.0

恢复位置 = true

自动字幕 = true
```

---

# 六十二、进度保存策略

为了减少数据库写入：

播放时：

```text
每5秒保存一次
```

并在：

```text
Pause

Item切换

页面Unmount

窗口关闭
```

时强制保存。

如果 position：

```text
< 5秒
```

可以视为：

```text
从头开始
```

不要因为用户播放1秒就破坏原进度。

---

# 六十三、播放结束策略

播放结束：

```text
position = duration

completed = true
```

Series：

重新计算：

```text
completed_items
```

---

# 六十四、学习库状态同步

下载完成事件发生时：

不要让前端轮询整个任务列表。

使用当前事件体系或新增：

```text
learning-library-updated
```

通知前端刷新。

避免高频全量查询。

---

# 六十五、迁移策略

v0.3 用户升级 v0.4：

必须：

```text
原下载设置保留

原任务保留

原历史保留

Cookie保留

Proxy保留

Media Pro设置保留

Preset保留
```

新增：

```text
learning.db
```

即可。

不能因为学习库数据库失败而破坏下载系统。

---

# 六十六、数据库失败处理

如果：

```text
learning.db
```

初始化失败：

应用仍可进入：

```text
下载

媒体工具

历史

设置
```

学习库显示：

```text
学习库初始化失败
```

不要导致整个映流无法启动。

---

# 六十七、v0.3 已知问题同步修复

v0.4 开发过程中同时处理：

## React StrictMode Preview Session

修复开发模式：

```text
首次模拟 cleanup
```

导致 Media Pro 预览会话失效的问题。

要求：

每次 effect 生命周期拥有自己的预览 session。

生产行为不得回归。

---

# 六十八、不要在本版本实现

明确禁止扩大范围：

- AI自动总结课程
- AI自动生成笔记
- AI问答
- OCR
- 语音识别字幕
- 向量搜索
- 云同步
- 账号系统
- 多用户
- 在线流媒体服务器
- Jellyfin服务器
- Elasticsearch
- Redis
- Docker
- 完整NLE
- 多轨剪辑
- 云端学习记录

这些属于后续版本。

---

# 六十九、建议参考 GitHub 项目

只借鉴：

```text
产品逻辑
交互方式
数据结构思想
```

不要直接复制不兼容许可证代码。

---

## TubeArchivist

重点参考：

```text
Playlist

视频归档

已看 / 未看

频道 / Playlist更新

本地播放

源站视频ID

下载与媒体库关联
```

---

## Jellyfin

重点参考：

```text
Continue Watching

媒体卡片

Series详情页

播放器布局

媒体库导航
```

---

## Audiobookshelf

重点参考：

```text
系列

章节

播放位置

继续播放

长内容学习进度
```

---

# 七十、UI 风格

继续使用映流：

```text
深色

简洁

Windows桌面软件风格
```

不要直接复制 Jellyfin / TubeArchivist UI。

保持现有：

```text
sidebar

cards

spacing

buttons

typography
```

统一。

---

# 七十一、响应式要求

重点测试窗口宽度：

```text
1600

1440

1200

1000

880
```

Series 页面在：

```text
880px
```

时：

课程列表可以移到播放器下方。

不要让视频播放器被挤压到无法操作。

---

# 七十二、性能目标

测试：

```text
100 个分类

1000 个系列

10000 个课时
```

要求：

学习库首页不能一次渲染所有课程。

使用：

```text
分页

或者虚拟列表
```

Series 内：

几百集教程不得明显卡顿。

---

# 七十三、数据库操作要求

SQLite：

使用事务处理：

```text
批量导入

批量排序

Playlist导入
```

避免：

```text
一集一次commit
```

导致大型 Playlist 导入很慢。

---

# 七十四、测试

必须增加 Rust 测试。

---

## Database

测试：

```text
创建数据库

迁移

分类CRUD

Series CRUD

Item CRUD
```

---

## Progress

测试：

```text
保存播放进度

恢复播放位置

完成阈值

手动完成

重新标记未完成
```

---

## Notes

测试：

```text
创建

编辑

删除

排序
```

---

## Import

测试：

```text
文件导入

文件夹导入

自然排序

重复路径

不存在文件
```

---

## Playlist

测试：

```text
PlaylistEntry → LearningItem

playlistIndex顺序

source_video_id去重
```

---

## Download Integration

测试：

```text
下载成功

↓

LearningItem更新file_path
```

以及：

```text
下载失败

↓

Item保留
```

---

# 七十五、前端测试

至少覆盖：

```text
学习库首页

分类创建

Series创建

本地导入

Series详情

播放器

继续播放

手动完成

时间点笔记

Playlist加入学习库
```

---

# 七十六、回归测试

必须继续运行 v0.3：

```text
npm test

cargo test
```

并保留：

```text
原下载界面测试

Media Pro测试
```

不能因为 v0.4 删除旧测试。

---

# 七十七、Windows 实机验收

必须实际验证：

## Test 1

新建：

```text
UE5
```

分类。

---

## Test 2

导入：

```text
10个本地教程视频
```

生成一个 Series。

---

## Test 3

播放第3课。

观看到：

```text
约50%
```

退出应用。

---

## Test 4

重新打开。

点击：

```text
继续学习
```

恢复时间位置。

---

## Test 5

播放达到：

```text
90%
```

自动完成。

---

## Test 6

播放结束：

自动进入下一课。

---

## Test 7

在：

```text
12:30
```

添加时间点笔记。

重新打开并点击笔记。

播放器正确跳转。

---

## Test 8

解析真实或离线模拟 Playlist。

创建学习系列。

顺序与：

```text
playlistIndex
```

一致。

---

## Test 9

同一 Playlist 再次导入。

不得创建重复课时。

---

## Test 10

本地视频被移动。

学习库能够显示：

```text
文件不存在
```

并支持重新定位。

---

# 七十八、文件保护验收

学习库操作不得修改原始视频内容。

测试：

```text
导入前 SHA256

=

导入后 SHA256
```

观看：

不得改文件。

写笔记：

不得改文件。

标记进度：

不得改文件。

---

# 七十九、Codex 开发顺序

严格按照以下阶段执行。

---

# Phase 1
# Learning Database

完成：

```text
SQLite

Schema

Migration

Repository

Models
```

先写测试。

---

# Phase 2
# Category + Series

完成：

```text
分类CRUD

Series CRUD

UI
```

---

# Phase 3
# Local Import

完成：

```text
文件导入

文件夹导入

自然排序

ffprobe
```

---

# Phase 4
# Learning Player

完成：

```text
视频播放

倍速

上一课

下一课

进度
```

---

# Phase 5
# Continue Learning

完成：

```text
自动保存

恢复进度

完成状态

自动下一课
```

---

# Phase 6
# Notes

完成：

```text
时间点笔记

编辑

跳转
```

---

# Phase 7
# Downloader Integration

完成：

```text
Playlist

↓

创建Series

↓

创建LearningItems

↓

DownloadTask

↓

下载完成绑定文件
```

---

# Phase 8
# Series Update

完成：

```text
检查Playlist更新

增量下载
```

如果 P1 工作量过大：

允许后移至：

```text
v0.4.1
```

但是：

数据库必须预留 source 信息。

---

# Phase 9
# Stabilization

完成：

```text
StrictMode修复

回归测试

Windows实机验证

性能检查
```

---

# 八十、验收标准

v0.4 完成后：

用户必须能够完整执行：

```text
打开映流

↓

解析一个系列教程播放列表

↓

选择：

下载后加入学习库

↓

选择分类：

UE5

↓

开始下载

↓

学习库自动创建：

UE5 Niagara完整教程

↓

课时按照源Playlist顺序排列

↓

下载完成后可播放

↓

播放到一半退出

↓

重新打开映流

↓

继续学习

↓

恢复上次时间点

↓

播放完成自动标记

↓

进入下一课

↓

添加时间点笔记
```

这条完整链路必须可用。

---

# 八十一、版本号

统一修改为：

```text
0.4.0
```

包括：

```text
Cargo.toml

package.json

tauri.conf.json

UI版本显示
```

不要出现：

```text
0.3.0 / 0.4.0
```

混用。

---

# 八十二、README

更新 README。

新增：

# 学习库

说明：

```text
教程分类

系列管理

本地视频导入

播放进度

继续学习

笔记

Playlist一键创建系列
```

增加学习库截图位置。

---

# 八十三、交付文档

新增：

```text
docs/learning-library.md

docs/learning-library-delivery.md
```

---

# 八十四、最终 Codex 输出

完成开发后必须给出：

## 1

修改文件完整列表

## 2

数据库设计

## 3

Migration说明

## 4

新增 Rust 模块

## 5

新增 Tauri Commands

## 6

新增 React 页面与组件

## 7

Downloader → Learning Library 集成方式

## 8

测试结果

包括：

```text
npm test

cargo test

frontend QA
```

## 9

Windows 实机验收结果

## 10

构建结果

## 11

已知问题

## 12

延期到 v0.4.1 / v0.5 的项目

---

# 八十五、实现要求

不要只写：

```text
方案

TODO

Mock

占位UI
```

需要直接实现可工作的：

```text
Learning Library
```

核心 P0 必须完成。

如果发现现有代码结构和任务书描述略有不同：

优先：

```text
尊重现有代码架构
```

而不是为了符合文档：

```text
重复实现已有能力
```

---

# 八十六、最终产品定位

v0.4 完成后：

映流不再只是：

```text
视频下载器
```

产品链路应该变成：

```text
发现教程

↓

下载

↓

自动整理

↓

加入学习系列

↓

本地播放

↓

记录学习进度

↓

继续学习

↓

时间点笔记

↓

媒体处理
```

最终形成：

# 映流

> 面向本地视频收藏、系列教程学习与媒体处理的一体化桌面工具。

---

# 八十七、后续路线预留

v0.4.1：

```text
学习库稳定性

Series自动更新

播放器体验优化

更多排序与筛选

学习统计
```

v0.5：

```text
字幕全文搜索

字幕转录

自动章节

课程摘要

AI学习笔记

视频内容搜索
```

v0.6：

```text
AI课程助手

基于字幕问答

知识点提取

复习卡片

Quiz生成
```

当前 v0.4 不实现这些 AI 功能。

先把：

```text
教程下载
+
系列管理
+
本地播放
+
学习进度
```

做稳定。

---

# 开始执行

请基于当前：

```text
codex/media-pro-v0.3
```

或项目中实际最新 v0.3 分支进行开发。

不要修改或覆盖已有 PR 历史。

新建独立开发分支，例如：

```text
codex/learning-library-v0.4
```

完成后：

创建新的 Draft PR。

不要自动合并。

不要发布 GitHub Release。

开发、测试、构建和实机验收全部完成后，再提交最终交付报告。