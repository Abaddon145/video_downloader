# Learning Library v0.4 design
Authority: learning-library-taskbook.md；用户明确直接执行该文档，完整实现P0及可复用的P1，不增加设计批准等待。

独立learning.db，rusqlite bundled（复用本机已有0.32.1），schema_version与顺序事务迁移。SQLite连接由Mutex保护；数据库错误只禁用学习库。旧DiskState、DownloadTask结构不改变。
分类→系列→课时；progress/notes外键级联只移除记录。Series完成统计由查询计算；本地规范化绝对路径去重、源课程优先ID再URL。全部批量写入使用事务。
Importer先扫描并预览（不写DB），确认后复用已有ffprobe、受控封面提取。最多500本地文件/10000目录项，分批可积累10000课时。文件不移动、不编辑；缺失可重新定位。删除文件仅在额外确认且没有其他课时引用时进行，报告失败并保留未删除记录。
独立学习播放器复用安全asset/可取消代理、文本字幕转VTT。5秒保存SQLite；timeupdate只同步内存游标，暂停/切换/卸载保存，Rust关闭窗口/主动退出刷新内存。保存快照序号防止迟到旧游标覆盖；90%默认完成、结束完成；倍速0.5/0.75/1/1.25/1.5/1.75/2。下一课倒计时5秒，可取消，隐藏或离页停止。
播放列表继续使用MediaPreview/PlaylistEntry；所有课时先建占位，勾选条目入现有队列，未选课时可稍后下载。独立command编排学习DB和原enqueue；冻结原下载参数并按来源/请求键绑定，回调与重启补偿修复跨存储窗口。失败保留课时、可重试；不按下载完成时间重排。
source信息支持手动检查列表更新，按ID/URL去重，仅选择新增条目下载；收藏与普通SQLite搜索覆盖分类/系列/课程/笔记。首页50系列/页、6条继续学习，课程50/页；100分类/1000系列/10000课程性能验收。
UI继续石墨灰#111318、深面板#191D23、青蓝#38BDF8、系统中文字体、Lucide图标。主页继续学习/分类/收藏/最近添加；详情播放器与课程列，<=1000宽课程下移。拖动排序用指针把手并提供上移/下移，保留原生外部文件拖入。技能设计库两次返回营销/儿童风格，与产品不匹配，采用现有设计作为fallback；采纳已验证拖动替代/React cleanup指南。
新增learning/{db,models,repository,importer,service,commands,tests}，前端features/learning页面/播放器/编辑对话框/导入/笔记/下载选项。避免为对齐示意图拆大量空模块。v0.3 StrictMode同时修复为每次effect独立session。
P2禁止扩展：AI、OCR、云同步、账号、服务器、完整NLE。
Reference: https://docs.rs/rusqlite/0.32.1/rusqlite/ ; https://www.sqlite.org/pragma.html#pragma_user_version ; https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/track
