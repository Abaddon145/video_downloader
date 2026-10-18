# Phase3 实际验证

普通cargo suite106通过，另有1项需要模型的测试默认ignored，已通过显式 `cargo test --locked whisper_real_ -- --ignored --nocapture` 实际执行。它运行Base、Small，对11秒JFK官方英语样本识别，校验中文/Unicode/空格/括号/#/&缓存路径，验证源文件只读锁、前后完整SHA一致，等待真正Whisper进程启动后取消并检查退出。完整结果phase3.txt。

6组真实桌面命令验收全部通过：缺模型、本地无Key转录、真实Base下载、Small下载取消重试、转录入库及音频清理、损坏模型拒绝/本地导入修复/错误SHA拒绝、无音轨视频。分组详见phase3-desktop.json。这些是本机Windows测试，尚不代表全部语言/设备矩阵。

固定[Whisper v1.9.1](https://github.com/ggml-org/whisper.cpp/releases/tag/v1.9.1) CPU x64；官方发行API摘要与实际下载哈希一致。模型来自[作者仓库](https://huggingface.co/ggerganov/whisper.cpp/tree/5359861c739e955e79d9a303bcbc70fb988958b1)，两文件均实际下载并与LFS SHA256一致，见whisper-model-verification.json。没有将SHA1误用为SHA256。模型不进安装器。

实际发现v1.9.1 CLI的Windows命令行仍用系统代码页；中文绝对参数曾导致进程直接退出。复现相对路径成功，并核对固定提交examples/cli/cli.cpp注释。应用现用受控缓存工作目录及ASCII相对audio.wav/model.bin/transcript参数，复用native::spawn_in与Job。模型用同卷硬链接，文件系统不支持时复制；模型验证与任务取消均有效。源视频不传给Whisper，不被修改。

重跑：设置AI_QA_FIXTURES到含官方jfk.wav、已核验ggml-base.bin/ggml-small.bin的本地目录，AI_QA_RESOURCES到src-tauri/resources。桌面脚本scripts/qa/verify-ai-transcription-desktop.mjs使用独立数据目录，PLAYWRIGHT_MODULE指向已安装Playwright，QA_APP可指定custom-protocol构建的可执行文件。此阶段没有真实DeepSeek付费请求。
