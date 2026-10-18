# 映流 v0.5.0  
# AI视频学习分析 + Obsidian兼容知识库 Codex执行文档

---

# 0. 项目定位

项目：

`Abaddon145/video_downloader`

当前基线：

`v0.4.2`

优先基于包含 PR #7 原生 mpv 学习播放器全部改动的最新代码开发。

如果：

`codex/native-learning-playback-v0.4.2`

尚未合并，则以该分支作为基线。

建议新分支：

`codex/learning-ai-knowledge-v0.5.0`

---

# 1. v0.5.0 核心设计原则

本版本最重要的产品决策：

## 不新增独立“AI分析”一级模块

AI分析不是独立工作空间。

AI能力必须直接融合进：

**学习库 → 系列 → 对应课程视频**

用户工作流：

```text
学习库
↓
打开课程
↓
播放视频
↓
AI分析
↓
查看总结 / 字幕 / 问答 / 笔记
↓
提取知识
↓
保存到知识库
```

而不是：

```text
学习库
↓
切换到AI模块
↓
重新寻找同一个视频
↓
查看分析
↓
再切回学习库
```

---

# 2. 最终一级导航

v0.5.0左侧导航目标：

```text
映流

下载
学习库
知识库
媒体工具
历史
设置
```

禁止新增：

```text
AI分析
```

一级导航。

---

# 3. AI在产品层和代码层的关系

必须遵循：

> 产品上融合，架构上分离。

即：

## 产品UI

AI属于：

`学习库`

## Rust架构

AI保持：

`AiService`

独立模块。

不要把AI所有逻辑塞进：

`LearningService`

---

# 4. 总体架构

```text
Learning Library UI
        │
        ├── LearningService
        │      ├── 课程
        │      ├── 播放进度
        │      ├── 笔记
        │      └── 字幕
        │
        ├── AiService
        │      ├── FFmpeg
        │      ├── Whisper
        │      ├── DeepSeek
        │      ├── Transcript
        │      ├── Keyframes
        │      ├── Summary
        │      └── Video QA
        │
        └── KnowledgeService
               ├── Markdown
               ├── Vault
               ├── Search
               ├── Wiki Links
               └── Obsidian
```

---

# 5. 最终用户体验

进入某个系列：

```text
UE5 Niagara教程

01 基础介绍                 ✓ 已分析
02 Emitter                  ✓ 已分析
03 GPU Collision            ● 分析中 64%
04 Renderer                 ○ 未分析
05 Optimization             ○ 未分析
```

课程项显示AI状态即可。

不要出现复杂AI任务信息。

---

# 6. 课程AI状态

状态：

```text
未分析
排队中
分析中
已分析
分析失败
分析中断
```

显示形式：

```text
○ 未分析

● 分析中 68%

✓ 已分析

! 分析失败
```

---

# 7. 课程右键菜单

增加：

```text
播放

AI分析
重新生成AI总结
重新完整分析

生成知识笔记
打开知识笔记

重新定位视频
打开文件位置
```

没有分析结果时：

隐藏：

```text
重新生成AI总结
打开知识笔记
```

---

# 8. Learning Player最终布局

桌面宽屏：

```text
┌───────────────────────────────────────────────────────┐
│ UE5 Niagara Beginner Tutorial                        │
├───────────────────────────────┬───────────────────────┤
│                               │ 总结 字幕 问答 笔记   │
│                               │                       │
│         视频播放器             │                       │
│                               │                       │
│                               │                       │
├───────────────────────────────┤                       │
│ ▶ 12:31 / 38:21               │                       │
│ Controls                      │                       │
└───────────────────────────────┴───────────────────────┘
```

建议：

```text
播放器约65%

右侧学习面板约35%
```

---

# 9. 右侧不要叫“AI”

Tab最终建议：

```text
总结
字幕
问答
笔记
```

不要：

```text
AI
```

单独作为Tab。

原因：

用户关心的是功能结果，而不是AI技术本身。

---

# 10. 总结Tab

未分析：

```text
还没有生成视频总结。

AI会在本地提取语音和关键画面，
并使用DeepSeek理解视频内容。

原视频不会上传。

[开始分析]
```

---

# 11. 已分析状态

总结Tab：

```text
一句话总结

本课程主要介绍Niagara GPU粒子碰撞、
Distance Field以及GPU Compute Sim设置。


内容概要

...


自动章节

00:00 课程介绍
03:42 创建Niagara
08:41 GPU Compute Sim
12:31 GPU Collision
18:45 Distance Fields


关键知识

...


教程步骤

...


专业术语

...


[重新生成总结]

[生成知识笔记]
```

---

# 12. 时间点必须可点击

例如：

```text
12:31 GPU Collision
```

点击：

播放器：

```text
seek(751秒)
```

AI结果必须与播放器深度绑定。

---

# 13. 字幕Tab

字幕来源：

优先：

```text
已有字幕
```

没有字幕：

使用：

```text
Whisper本地转录
```

显示：

```text
00:12:29
这里我们把Niagara Sim Target切换为GPU Compute Sim。
```

当前播放位置自动高亮对应字幕。

点击字幕：

跳转到对应时间。

---

# 14. 问答Tab

用户可以直接问：

```text
这个视频什么时候讲GPU碰撞？
```

返回：

```text
视频在12:31开始介绍GPU Collision。

重点包括：

• GPU Compute Sim
• Distance Field Collision
• Collision Radius

[▶ 12:31]
```

---

# 15. 视频问答必须基于当前视频

问答上下文必须固定：

`当前 item_id`

禁止用户在一个视频里问问题，却检索其他课程内容。

未来可以再做：

`整个系列问答`

但v0.5.0不做。

---

# 16. 笔记Tab

保留现有时间点笔记。

升级为：

```text
普通笔记

截图笔记

AI整理笔记
```

---

# 17. 截图笔记

增加按钮：

```text
📸 截图并记笔记
```

点击：

1. 获取当前播放时间
2. FFmpeg截当前帧
3. 保存附件
4. 创建时间点笔记
5. 用户输入文字

例如：

```text
12:31

[截图]

GPU Collision需要切换到GPU Compute Sim。
```

---

# 18. AI解释当前片段

播放器增加：

```text
✨ 解释这一段
```

发送：

```text
当前时间 ±30秒字幕

+

当前附近关键帧
```

给DeepSeek。

禁止重新分析整个视频。

---

# 19. AI分析启动弹窗

用户点击：

`AI分析`

显示：

```text
分析视频

UE5 Niagara Beginner Tutorial


分析模式

○ 快速

● 标准

○ 深度


本次处理：

✓ 本地语音识别
✓ 自动章节
✓ 视频总结
✓ 关键知识提取
✓ 关键画面分析


发送给DeepSeek：

✓ 字幕文字
✓ 精选关键帧


不会上传：

✓ 原视频
✓ 完整音频


[开始分析]
```

---

# 20. 三档分析模式

## 快速

适合：

快速知道视频讲什么。

建议：

```text
完整转录
少量截图
约180秒兜底
最多12张关键帧
```

---

## 标准

默认。

```text
完整转录
场景变化检测
约90秒兜底
最多24张关键帧
```

---

## 深度

```text
完整转录
更密集场景检测
约45秒兜底
最多60张关键帧
分章节分析
二次全片总结
```

这些值：

集中定义。

禁止散落硬编码。

---

# 21. DeepSeek角色

DeepSeek只负责：

```text
理解
总结
分类
解释
问答
```

不负责：

```text
视频解码
音频识别
直接处理MP4
磁盘路径
数据库操作
```

---

# 22. 视频分析真实数据流

```text
视频
 ↓
FFmpeg
 ├── 音频
 └── 关键帧
 ↓
whisper.cpp
 ↓
时间轴Transcript
 ↓
分块
 ↓
DeepSeek
 ↓
Chunk分析结果
 ↓
Final Summary
 ↓
SQLite
 ↓
Learning Player
 ↓
Knowledge Vault
```

---

# 23. 禁止直接上传视频

禁止：

```text
MP4 → DeepSeek
MKV → DeepSeek
WMV → DeepSeek
WAV → DeepSeek
```

允许：

```text
字幕文字

+

精选JPEG截图
```

且必须经过用户设置授权。

---

# 24. DeepSeek默认模型

默认：

```text
deepseek-flash
```

模型名称不能散落硬编码。

统一配置。

未来允许更换。

---

# 25. DeepSeek API

Rust