# 映流 v0.5.0 交付与验收

状态：v0.5.0 候选安装包已构建并在本机更新安装，PR 保持草稿，没有发布正式 Release。AI 位于学习库对应课程下，不单独建立一级导航。此文按实际执行结果持续更新。

| 项目 | 真实证据与范围 |
| --- | --- |
| 基线 | [Phase0](evidence/ai-v0.5.0/baseline.txt)，v0.4.2 mpv 原生播放器保留 |
| 数据迁移 | v5→v6、空库、旧笔记/进度保留、未来版本拒绝；普通 Rust 测试 |
| Rust / Node | 最终172通过、0失败、8项显式集成测试默认忽略；Node37通过。新增付费真实API和本地时长/长路径矩阵均已单独通过。[结果](evidence/ai-v0.5.0/phase12/) |
| DeepSeek真实服务 | 官方HTTPS，deepseek-flash，一段公开演讲短片；1次分析（分段+全片）和1次问答，2290输入/1072输出Tokens、256缓存Tokens；1关键帧、1章节、带时间引用、生成3份Markdown，源SHA一致，DPAPI测试Key已删除。[脱敏报告](evidence/ai-v0.5.0/phase12/live-deepseek.json) |
| DeepSeek模拟服务 | 401/402/429/5xx、超时、断网、截断/非法/过大JSON、重试、取消、恢复、上传正文仅获准文字/JPEG。见Phase2/5/6日志；不把模拟结果称为真实服务质量 |
| Whisper | 真实Base/Small固定SHA下载、取消重启、损坏拒绝、本地英文转录、无音轨跳过；[Phase3](evidence/ai-v0.5.0/phase3-desktop.json) |
| 关键帧 | 12种真实格式、HEVC、4K缩放、无音轨、场景检测、源SHA不变；[Phase4](evidence/ai-v0.5.0/phase4-real.txt) |
| Vault安全与兼容 | 普通Markdown/YAML/Wiki/标签/附件/索引、反向链接、路径与Junction边界、外部修改冲突、来源文件只读；[Phase8](evidence/ai-v0.5.0/phase8/acceptance.md)、[Phase9](evidence/ai-v0.5.0/phase9/acceptance.md) |
| 课程整合 | 原生mpv跳转、已有字幕优先、分页跟随、MediaPro/下载入口、下载自动分析默认关闭；[Phase10](evidence/ai-v0.5.0/phase10/) |
| 知识笔记 | 默认自动生成、版本复用保留改稿、移动关联恢复、截图准确时间、当前版本联合删除需预览+确认；真实Windows桌面9/9、真实FFmpeg截图1/1；[Phase11](evidence/ai-v0.5.0/phase11/acceptance.md) |
| 窄屏 | 课程/知识库880px及1440px交互和截图复核；知识库18项和课程34项浏览器IPC模拟均通过。125%/150% deviceScaleFactor 浏览器模拟均通过，分别保留截图；未据此声称实体多显示器DPI验证 |
| 旧功能实机回归 | 发布程序真实MediaPro16/16、mpv13/13、真实yt-dlp本地HTTP播放列表/兼容播放8/8；浏览器下载UI13/13、原生UI9/9、MediaPro竞态UI4/4。本地HTTP不等同于当日B站/YouTube源站可用性保证。[证据](evidence/ai-v0.5.0/phase12/) |
| 时长/路径/多语言字幕 | 真实10秒/10分钟/61分钟无音轨视频、380字符中文/空格/括号/#/&/Ω/韩文路径，四类字幕均保留，关键帧有界，源SHA不变。未调用AI，也不把字幕处理称为多语言语音识别质量测试。[报告](evidence/ai-v0.5.0/phase12/local-duration-language-matrix.json) |
| 版本 | package/Cargo/Tauri0.5.0；界面通过Tauri getVersion读取，预览/失败不显示伪安装版本；[检查](evidence/ai-v0.5.0/phase12/app-version-ui.json) |
| NSIS /安装 | 中文x64 NSIS构建成功（187458656 bytes），本机0.1.1→0.5.0更新安装退出0；旧安装和私有数据已备份，比较到的原JSON/SQLite/key文件SHA不变，无丢失文件。隔离资料启动已安装程序，界面显示实际0.5.0、原生播放及真实AI结果/知识预览正常，工具摘要一致。[安装报告](evidence/ai-v0.5.0/phase12/installation.json)、[显示验收](evidence/ai-v0.5.0/phase12/installed/installed-acceptance.json) |

当前限制：未声称真实Obsidian程序联用、干净Windows10缺WebView2、所有GPU和实体多显示器DPI组合已验证。短韩语和拼接中英语音已实际执行并发现识别局限，60分钟以上真实课程的整套AI质量尚未验证；多语言字幕/超长媒体的结构处理检查与语言识别质量分别记录。知识库大纲对超过256K字符的笔记停用同步生成，笔记仍可编辑和搜索。CodeMirror/Markdown延迟加载模块约805KB，保留Vite体积警告。

测试密钥只用于明确授权的小规模验收，未进入源码、React、普通设置、学习数据库或诊断；真实测试进程只在Rust中解密并调用固定官方域名，结束后删除测试凭据。没有上传原MP4/MKV/WMV或完整WAV。源视频SHA前后一致。

组件版本、固定来源、归档SHA与许可见 src-tauri/resources/components.json、ai-components.json、learning-components.json、licenses/ 和生成的 tools/binaries.json。Whisper模型不在安装包中；首次下载或导入时必须验证固定摘要。

## 文件核验与审查

安装包 SHA256：`c41749ebebd65c54fecb1d513c446dc72891701983aa3c18c22b288c235ad177`。Tauri给安装载荷的可执行文件写入`__TAURI_BUNDLE_TYPE_VAR_NSS`，普通release构建保留`UNK`；只替换这一个固定标签后，整个文件字节相同。初次简单比较整文件hash失败后已核对本地Tauri源码规则，未忽略任意偏移或其他差异。

396个源码/前端dist/文档证据/SQLite/状态/日志文件的DeepSeek明文凭据签名扫描无匹配，测试DPAPI Key已删除。[扫描](evidence/ai-v0.5.0/phase12/credential-scan.json)。原视频从未写入AI请求；模拟HTTP检查授权内容过滤，真实测试验证生产Rust链路。

独立审查以v0.4.2基线`806fe45caa4a7ffa2981c63358e5accd786bbbb0`核对关键完整源码差异，未确认阻止交付的可复现P1/P2；静态审查与主代理实机回归分别记录。[审查范围](evidence/ai-v0.5.0/phase12/independent-review.md)。未使用GPT-6 Astra。

旧UI回归夹具曾对新增AI命令统一返回布尔值，导致模型列表渲染中断；改为正确的空设置/模型/用量结构后，原下载、Cookie、更新、预设和快捷键断言全保留并通过。旧笔记入口改为先选择课程“笔记”标签。安装版验收先等待组件校验完成再导入。以上是夹具/同步修正，没有删除旧测试或降低产品保护。

## 多语言短语音质量：已执行，但不能算全部通过

真实本地Whisper Base和Small均完成中文对话、1.1秒韩语孤立词和“中文对话+6秒英文公开演讲”拼接样本的处理；无API请求，原文件SHA一致。[Small结果](evidence/ai-v0.5.0/phase12/local-speech-matrix.json)、[Base结果](evidence/ai-v0.5.0/phase12/local-speech-base.json)。

中文对话有可用转录。韩语参考词为`우리`，Small自动检测韩语但输出`꾸리`，Base误判为日语。两种模型对拼接样本只输出中文部分，未保留后段英文。因此这些质量断言未通过；不能宣称多语言语音准确度验收完成。样本很短且混合样本是拼接，不能推广为所有韩语/双语课程均失效，也不能隐藏本次结果。已有可信同名SRT/VTT等字幕会优先读取，是当前可用的绕过路径；自动识别结果需人工核对。

素材来源及许可记录在JSON中：中文原录音[Commons](https://commons.wikimedia.org/wiki/File:Chinese-Lesson1-Dialogue1.ogg)，韩语原发音[Commons](https://commons.wikimedia.org/wiki/File:LL-Q9176_(kor)-Daniel_Mietchen-%EC%9A%B0%EB%A6%AC.wav)。音频和模型未加入源码ZIP或安装包。首次脚本在报告已写完后因GBK控制台无法打印韩文退出1，后改为ASCII转义输出；引擎进程均实际退出0，质量不匹配仍按上述失败记录。
