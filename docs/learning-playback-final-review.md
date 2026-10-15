# v0.4.1 最终独立审查与修复记录

审查范围：`c580e5d7d0e20ed17457a18ecf590876f5ec1ec0..f7856cad1394f93ea4bdcb12be31ecb9b68013e6`。独立只读审查一次，之后按计划集中进行一次 Important 修复与回归验证，没有再启动第二轮审查。

首选审查模型因账户额度立即失败，未产生审查；同一次审查改由可用的 gpt-6.1-sol 完成。Critical 0、Important 3、Minor 0、Declined to judge 0。三项 Important 均接受并修复，未推迟有效发现。

## 1 旧手动缩略图迁移

问题：v4 库中缓存目录外的本地手动图片在 v5 中被隐藏，随后可能被自动图片代替。

修复：普通本地 JPEG/PNG/WebP 在校验后保留可见性，后台复用手动选图流程复制、标准化至受管缓存；始终保留手动优先级，不修改原图。

实际 Windows RED：升级后首次读取 thumbnailPath 为空，迁移检查 0 通过、1 失败。GREEN：同一真实 v4 数据库迁移检查 1/1，通过原图 SHA256、manual 优先级和缓存归一化断言。脚本：`scripts/qa/verify-learning-migration.mjs`；证据：`learning-playback-evidence/migration.json`。

## 2 抽帧期间重新定位

问题：A 的工作执行期间课程定位到 B；B 的入队被去重丢弃，A 结束后错误地将 B 标为已尝试，导致 B 不再自动生成。

修复：结束时只记录本次真正尝试的签名；最新输入发生变化则重新排队一次。输入未变的失败继续记忆，避免无休止重试。

RED：`relink_during_paused_thumbnail_work_requeues_latest_input_once` 无待处理任务而失败。GREEN：该测试及 `unchanged_thumbnail_failure_is_not_rescheduled_forever` 通过，真实缩略图检查 4/4。

## 3 完成到缓存发布之间的重复转换

问题：FFmpeg 已完成、info.json 尚未发布的窗口中，迟到请求无法复用原任务，可能重复转换并竞争发布。

修复：Completed 但尚未发布的同源任务继续共享同一个发布锁和结果；发布失败允许重建。校验任务归属共享任务，最后一个引用取消时终止；客户端取消不能导致其他引用失效。授权操作与会话检查在同一受控范围内。

RED：暂停发布边界后 `completed_proxy_stays_shared_while_finalization_is_paused` 拒绝复用并失败。GREEN：该检查、失败后重试检查通过，实际三请求共享/单请求取消/最后引用取消通过。

同一修复过程中补充 force 边界 RED→GREEN：完成发布后的手动重新生成必须创建新任务，发布中的并发请求仍应共享。`force_rebuild_shares_inflight_publication_but_rebuilds_a_published_proxy` 通过。

## 验证与裁定

修复后 Rust 全套 91/91；生产程序重新构建；真实 Windows 迁移 1、多格式播放 19、异常场景 3、缩略图 4 项全部通过。Node 22/22、前端生产构建、既有 UI 46 项和其余桌面回归保持通过；详见交付报告和证据目录。

- Ruling：稀疏 TS/M2TS 关键帧导致快速输入定位无图时，精确输出定位仅重试一次。保留常规抽帧速度，最多多一次有 30 秒上限的解码；再次失败使用封面或默认图。
- Ruling：首选模型额度不足后，同一次独立审查改用可用模型；没有首选 gpt-6-astra 的审查意见。
- Minor：无；Declined to judge：无；未解决 Critical/Important：无。
