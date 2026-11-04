# #1393 编码前复用记录

名分：绑定 #1393 的工作草稿；CI 文件是本票待接线的验证入口。开工时指定三个代码路径及本目录均不存在，checkout 干净，HEAD 为 ab3f3fb0e1638f1ef3543b8b1948cd1856dc8675。

已读 CLAUDE/AGENTS、ask-matt、Wayfinder、delivery/generation、CODING_STANDARDS、本票 REPORT/INDEPENDENT-REVIEW-01、仓外 VALIDATION-CONTRACT-DRAFT。沿已授权实施范围直接实施；不启动其他工蜂、不重新裁 W、不操作 tracker。

- `.github/workflows/ci.yml`：保留现有所有 job/step；复用 ubuntu-latest、60 分钟总上限、CARGO_INCREMENTAL=0、jobs=1、官方 elan 安装方式；仅给 fixture job 增加明确的 Lean 单文件检查。
- `.github/scripts/run-cargo-with-telemetry.sh`：原样复用 2147483648 字节起始磁盘门和 10 秒采样。输出分别保留，实际命令退出单记。它不提供 OS 内存/store 硬帽或全局截止。
- `s_session/tests/tb02d_harness.py:execute`：借鉴独立进程组、TERM→KILL、实际 wait/退出及清理错误原件；不照搬每 run 重置 900 秒或事后新增截止。新监督器从 workflow 首步起同时继承 monotonic 与墙钟起点，所有子命令共用一个剩余窗口，回收和封包预留在 60 分钟内。
- `report/reference-evidence`（实际路径为同票 issue1393-sandcastle-20261018/reference-evidence）：读取管理者已有 JSON 原件，三个索引结果及 Nautilus 流式状态材料只作工程参考。保留“一个状态跨切片”的寿命原则用于共享截止，不借用 #1419 业务实现或 Nautilus 业务判据；没有联网。
- 源码实际核对：新增 lib 4 个位于 inclusion 的 tb02u_tests 模块、1 个位于 parser 的 tb02u_tests；两个旧核回归位于 inclusion::tests；两个 S 测试 include 到 s_structure_session::tests。最终完整名仍须 Cargo 当前 binary 的 list 证实，静态名称不是运行证据。

当前工具集中未提供 codebase-memory 图工具；已在不可用后采用定向文本检索。原始合同的产品 source 已被独审阻断，不复用其执行许可；本次文件维持 False，最终绑定由管理者返修收口后提供。
