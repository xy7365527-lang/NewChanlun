# #1394 cfg(test) 收益 pilot：外盘准备阻断

状态：**NOT_RUN**。固定候选为 `c6255d308fc69d94178916a1dc5534546fb77591` / Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395`。隔离诊断工位在同一提交，仅新增 `cfg(test)` 观测与 ignored 测试，当前完整诊断 patch SHA256 为 `e18ffbaefceb9c577f00c92bfed9edb8821e498afce481f9a90011f0f8d13ff3`。Standards PASS 已到；未执行收益 testbin build，也未运行两次 audit 或同外盘 baseline。

宿主磁盘空闲约 206–224 MiB，低于自有 build 的 250 MiB 安全停线。拟用独立外盘 `/Volumes/AgentStorage/NewChanlun-1394-reuse-pilot-20261001` 放完整跨设备副本、target、TMPDIR 与所有新结果。该卷为已挂载 local APFS、非只读，diskutil 报约 638.6 GB 空闲，但唯一 owned 目录创建两次阻塞：Python `os_mkdir` 进程 PID 60994 在内核等待约 5 分 38 秒，收到 SIGTERM 后由工具会话回收、exit 143；独立 `/bin/mkdir` 在 10 秒未返回，SIGTERM 后 `communicate()` 回收、exit -15。两次后目录均不存在。无 `cp`、build、audit 子进程残留；没有做 native UI 提示检查，不能判断是否有系统授权弹窗。

跨卷脚本 `prepare_external.py` 将在可写后先核候选当前 HEAD、Rust tree 与冻结五源 SHA，再 `cp -pR` 候选 target、旧诊断 owned A DB/WAL/SHM 至两份独立外盘源，逐文件核 SHA 与 `(dev,inode)` 隔离。`build.py` 从 300 MiB 稳定余量启动，只采样外盘产物目录的 250 MiB 停线，另记宿主 free；异常时对唯一自有进程组 TERM/KILL/wait。`run_baseline_external.py` 与 `run_probe.py` 分别绑定旧/新 testbin，在同外盘各运行两次只读 `state::audit`，返回值四成员及每 cut 五成员、24 表/48,403 行/96,397 BLOB、578 marker、SQL found/F 与角色根计数按脚本 gate 逐项核。当前这些均只是准备，不是已通过的动态结果。

本工位未改 main、未 push、未写 tracker、未动旧 target/原源。旧正式 A 的 `final.fullDB.json` SHA256 固定为 `8a7f8f248d1ffad4081555d2900bfd9bc73fb32971167edb544cb15292ea8174`，旧 shadow `PROBE.json` SHA256 固定为 `e5081eb994bbd3109d341292b0816d75e24d60aaabc802030efbdb4cf09643b9`；不对活动 root 当前源码施加旧 HEAD 不变要求。full9011、整图和本 cfg(test) 收益计数均未运行。
