# v0.4.0 最终独立审查记录

审查基线 `50aa2a9` → `4b4a5b6`，模型 gpt-6-astra，独立只读审查一次。原结论为 **With fixes**。Critical 0、Important 4、Minor 0。审查没有修改源代码或索引，也没有执行个人 Cookie 或 B站/YouTube 实时下载。

审查肯定了 SQLite 事务迁移、记录级联删除、共享文件保护、自然排序、分页、原下载参数冻结及预览会话隔离。以下四项全部接受，在同一修复阶段完成，无二次审查。

| Important 发现 | 失败复现 | 修复与验证 |
| --- | --- | --- |
| 普通下载任务早于课时建立时无法绑定；完成回调和重启补偿同样遗漏 | `learning_reuses_earlier_download_without_matching_old_history`：任务1000、课时2000，绑定 ID 为 NULL；另补较晚导入课时不可误绑旧历史的检查 | 明确关联使用课时 ID + 请求键；自动补偿允许排队/活动/暂停任务，终态仅匹配完成前建立的课时。Rust RED→GREEN；真实 yt-dlp 较早暂停任务在导入后复用，完成后可用且队列无重复 |
| 重新定位后当前播放器仍停在缺失或旧文件状态 | `relink remounts the current missing player without leaving the lesson`：定位后等待当前视频可播放超时 | 文件定位事件使当前播放器换用新会话，清理旧会话并停止旧进度写入。界面 RED→GREEN；Windows 真文件移动、定位后当前视频直接可播放，笔记保留 |
| 暂停改倍速后五秒内托盘退出会保存旧倍速 | `paused rate and seek changes publish fresh progress before the five-second timer`：1 !== 1.75 | `ratechange` / `seeked` 立即发布有效快照。界面 RED→GREEN；发布程序暂停改为1.75×并定位500秒，五秒定时器前经与托盘相同的原生保存路径退出，重开位置和倍速均保留。没有声称自动点击过托盘菜单 |
| 移除当前课时后播放器继续播放并保存不存在的记录 | `removing the current lesson stops playback and skips progress against the deleted row`：播放器仍可见 | 移除成功使当前会话失效并清空播放选择，停止倒计时/播放，卸载时跳过已删除记录保存。界面 RED→GREEN；Windows 播放中通过界面仅移除记录，播放器退出，本地文件保留 |

完整后台测试73/73、Node21/21、新增生命周期界面3/3通过。Windows学习库6+8+4项和原Media Pro16项通过；生产前端和中文NSIS构建通过。最终运行路径与数据见 `learning-library-evidence/*.json` 及执行记录。

测试脚本调整记录：HTTP离线夹具在解析阶段即可暂停；不再等待被故意阻塞的文件响应完成解析。浏览器定位检查等待实际快照事件（上限2秒），替代并行负载下不足的固定150毫秒延时。这些是夹具修正，软件修复均保留了先失败后通过的证据。

Rulings I made：无。Deferred minors：无。AI总结、OCR、云同步及多轨编辑是原任务书排除的后续范围。
