# v0.4.1 学习播放与缩略图设计

用户提供完整任务书并要求直接执行；沿用本任务已确认的本地执行、源码/安装包、新Draft PR交付方式。基于c580e5d，复用当前隔离目录，在统一codex/learning-playback-thumbnail-v0.4.1分支开发。

目标：点击本地课程时自动判断Direct/Remux/Transcode，兼容版本完成后自动播放；独立后台逐步补齐320×180 JPEG缩略图及课程状态。保留原视频、课程身份、进度、笔记、排序、收藏及下载关联。

## 播放
compatibility.rs封装基于ffprobe的计划、文件指纹、代理复用和生命周期。保留learning_prepare_player入口和PlayerMedia.path；添加计划和准备事件。MP4/MOV H264 8bit + AAC/MP3优先直接；WebM VP8/VP9 + Opus/Vorbis直接；兼容编码但容器不合适时只复制首个普通视频和首个音频到MP4/WebM。HEVC结合当前浏览器8/10bit能力；不能确认或实际播放失败时准备H264/AAC兼容版本，失败不无限重试。

浏览器能力使用实际navigator.mediaCapabilities.decodingInfo支持结果，参见[MDN](https://developer.mozilla.org/en-US/docs/Web/API/MediaCapabilities/decodingInfo)。直接播放失败可依序提升兼容等级，最终转码仍失败则提供定位/文件夹/技术详情。

代理直接使用现有MediaService任务、结构化MediaRequest、Windows Job、GPU检测与失败CPU回退；增加内部learning_playback标记，使仅播放用途的Remux只选择首个AV轨并在等待队列前部执行。后台缩略图用已有auxiliary，不占播放器任务队列。兼容输出H264/AAC、CRF22、AAC192k、faststart；不会修改原文件。进度/ETA沿用MediaTask。

learning-cache/playback/{SHA256}/保存完成的视频及info.json。源指纹=规范路径、大小、纳秒mtime；键还包括探测编码、容器和目标配置版本。请求按键去重；会话记录引用，取消最后一个请求可取消生成，其他请求仍引用时不取消共享任务。完成校验源指纹及输出时长后原子保存元数据，重启可命中；文件变化失效。

默认播放自动、50GiB上限、30天未用清理；设置可改。清理跳过等待、生成和当前播放引用，删除仅限经验证的本应用缓存。代理删除不触碰课时、进度、笔记或缩略图。仍允许完整代理完成后播放，本版不引入流式服务器。

## 缩略图
thumbnail.rs使用一个后台工作线程和按item_id去重的队列。手动>远程缓存>源视频抽帧>系列封面>占位。复用thumbnail_path，v4→v5只增加实际使用的thumbnail_source/source_thumbnail_url/thumbnail_key；老远程路径迁到source_thumbnail_url，旧本地图片按手动来源保留。

远程获取复用现有curl/代理解密与native受控进程，限制HTTP(S)、重定向、30秒及20MiB，之后用已有FFmpeg标准化。远程失败不影响课时，按源视频回退；没有文件则封面/占位，避免自动失败重试循环。JPEG320×180保持比例，scale+pad，抽帧时间30秒以上12%、5–30秒25%、不足5秒50%。本地指纹变化使generated失效，文件缺失保留已有图片；手动图片保留最高优先级。写入前核对最新行/来源，迟到任务不覆盖手动设置或重定位后新图。

导入、播放列表、下载完成、更新、重定位、列表读取和手动重生成会排队补图；导入不会等所有图完成。手动JPEG/PNG/WebP先标准化到应用缓存，用户原图不移动或删除；覆盖手动图需确认。清理仅删除数据库不再引用、且没有生成任务使用的缓存。

## 界面与数据
沿用石墨灰/青蓝及系统字体。LessonRow/LessonThumbnail拆出课程行；宽128×72、窄96×54，标题最多两行，lazy图片、时长角标、观看进度、完成小徽章、播放高亮、下载及缺失状态；图片失败回退封面/占位。操作收纳在原生details菜单，排序保留手柄和上下按钮，避免420px侧栏挤压。准备区提供进度、ETA、取消；高级操作保留重新生成兼容版本及技术详情。设置显示两类实际缓存占用，GPU设置仅引用Media Pro。

本地普通文件经ffprobe检测实际视频流，扩展名只用于选择器/文件夹候选；显式导入不因改名的扩展名拒绝，播放与下载绑定同样按实际探测。字幕SRT/ASS/SSA/VTT及内嵌文本轨独立转VTT并缓存，不为字幕转码视频。

## 验证与边界
逐阶段RED→GREEN与完整套件；最后一次独立全分支审查。生成多数本地格式夹具，WMV3/WVC1使用[FFmpeg WMV9样本](https://samples.ffmpeg.org/V-codecs/WMV9/)与[WVC1样本](https://samples.ffmpeg.org/V-codecs/WVC1/)仅本地验收，不捆绑第三方视频。验证混合格式、seek/rate/恢复、缓存命中/变化/清理、离线远程图、比例、手动图保护、取消共享任务、500集分页、100/1000/10000库、五窗口宽度和125/150%DPI模拟。记录实际验证限制，不替代真实结果。版本0.4.1，新草稿PR，不合并/Release。排除任务书列出的AI、云、多用户、HLS和完整NLE。
