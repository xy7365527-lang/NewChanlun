# #1394 Controller Q3 入口 revision1 Spec 增量复核

结论：PASS。旧 SPEC-01 HIGH 已修复，本次无新增 Spec 问题。该结论仅覆盖代码 Spec，不作工程或真实运行验收。

候选 `71e473bd54f46c31159a742a2b3ac706ac04a7da`；增量基线 `88b013213526041534cdfe1d16e9ea0753684922`。源 SHA `bac0956929d861b8a359ac187e2fd1d2dbd6adb96f2271701f9eaca860c6f1f9`；测试 SHA `82618c3338fd8187cbde292265cff02762c784e1d34a77b270531ee4b173e000`。两者与交接一致，受跟踪文件干净。旧 FAIL 的 MD、JSON、probe 与输出逐项核原 SHA，全部未改。

本轮只复核读取期限修复。原版已通过的 CLI、Q21 冻结、economic 拒绝、Ready 等事项沿用固定源审查，不重开无变化部分。批准源 C §5.6.1 第 311-313 行仍要求 exact Q21、无默认和启动时冻结；读取总期限来自本次已批准的实施补充合同。

- `s_service_control.py:473-480`：reader 使用 `O_NONBLOCK` 打开并对实际 fd 核 `S_ISREG`，FIFO 无写者不再等待。独立真实 FIFO probe 用 200 ms 预算，24.215 ms 即 ValueError，子进程 exit 2，`waitpid` 确認已回收，无 Q 或环境调用。原反例关闭。
- `:488-495`：读取与严格 JSON 解码均在本次直接子进程内，由 `subprocess.run(timeout=remaining(deadline))` 约束。本机 Python 的 POSIX run 实现已读，超时调用 kill 再 wait。独立 probe 在真实子进程的解码函数体内注入 CPU 死循环，并用 stderr 标记确认已进入；不是仅在启动前 sleep。150 ms 原预算先消耗 60 ms，run 只获 82.015 ms，155.074 ms 含调度及 kill/reap 开销后报 TimeoutExpired。子进程返回 -9，`waitpid` 确认已回收，无环境、Q 或 Ready 调用。没有宣称 OS 调度及回收零开销。
- `:502-526`：reader、environment、start_process、wait_ready 四处传入的绝对 deadline 逐值相同。正常 reader 消耗时间后，余下阶段没有获得新预算。
- `:483-498`：真实子进程返回 q3/legacy/unsupported 分类标记，父进程只接受三个固定字节串；额外 stdout 字节明确拒绝。重复键、非对象和破损 JSON 拒绝。未知及 null schema 归 unsupported，现有 start_q 拒绝该类别。

小标记只回答入口分派问题。空 limits 能返回 q3 标记，意为应交给 Q3，绝非认为 Q21 有效。`start_q:510-512` 仍把原 resource-config 路径及显式 s-session/3 交给先前核过且未变的 Rust QResources loader；具体 21 项 exact、正整数、冻结规范 SHA 都留在那里。因而此次修复没有改写配置、生成默认值或复制宽松 Q21 判据。

11 项独立 probe 全过，实际启动的 11 个进程均为本地元数据 reader，均已回收。Q 启动、环境调用和 Ready 在需要观测的 probe 中使用桩，未运行 Rust/Q3 服务。作者同字节 GREEN 30/30 复用；新跑 diff-check 为 0，两文件 AST 解析通过。ruff/mypy/pylint/black 不在可执行路径且无模块，未安装。

AST 增量比较只见 `start_q` 修改、新增 `query_resource_revision`；原 status/nonce/epoch/session/PID/Ready/stop 等方法无变化。镜像门由根代理报告已执行，本审查没有冒称独立运行。工程、真实 Controller/Q3、完整客户端、全资源、9011、整图和合入批准均不在本 PASS 内。

复现：`PYTHONDONTWRITEBYTECODE=1 python3 .scratch/issue1394-q3-controller-entry-20261001/revision1/SPEC-probes.py`。

产物为本目录 `SPEC-REVIEW.json`、`SPEC-probes.py`、`SPEC-probes-output.json`；输入及产物 SHA 索引见 `SPEC-REVIEW.sha256`。
