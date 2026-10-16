# v0.4.2 最终独立审查与处置

一次独立整分支审查，范围 fc1fa80..5efb7e5，由 fresh-context gpt-6-astra 完成。没有追加复审。

| 级别 | 问题 | 处置与证据 |
|---|---|---|
| Important | 取消状态校验和声明新会话分离，迟到旧 start 可停止新课 | 同一 runtime 锁内校验取消并声明所有权；native_cancelled_claim_preserves_newer_owner RED→GREEN；真实迟到取消检查通过 |
| Important | 内核故障后保存进度失败阻断手动转换 | 原生保存尽力执行后仍允许用户主动转换；manual_compatibility_remains_available_after_native_failure RED→GREEN |
| Important | 播放错误仅退出轮询，未释放活动窗口 | 发出最终错误后释放操作锁，并按会话归属结束；真实模拟内核退出时，窗口未回收 RED，修复后自动回收 GREEN |
| Important | 原生 Space/方向键绕过倒计时取消 | 可信输入配置将操作交给现有学习控件；绑定测试和倒计时检查 RED→GREEN |
| Minor → Important | 全屏控件焦点下 Escape 不退出 | 按键盘控件一致性重新分级，在控件过滤前处理 Escape；真实界面回归 RED→GREEN |

无待修复的 Critical/Important，也无延后的 Minor。

审查明确没有判断的部分：最终安装包和链接由交付步骤核对；不同实体 GPU、多显示器和系统 DPI 未全面实测，已在验收报告标明；许可接入按固定组件来源和上游文本核对，不出具法律意见。

FFmpeg latest 已变化造成固定归档摘要失败：保留严格拒绝策略，改用同提交固定发行归档，并核对原两个运行文件 SHA 完全相同。
