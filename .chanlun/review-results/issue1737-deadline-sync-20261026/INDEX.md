# #1737 测试同步修复交付索引

本目录是 #1737 的工作草稿，活期绑定该票。名分与月度出仓规则见 `docs/agents/generation-constitution.md` §5。

## 已合入的修复与证据

[PR #1738](https://github.com/xy7365527-lang/NewChanlun/pull/1738) 已于 2026-10-26T08:31:31Z squash 合入 `main`，落地提交为 `d310c46dd2710c6faf3c946e8e3e05b48f1034de`。评审与候选运行绑定 `a053ca2322ac9ab8d27d98e376f9c569024046bb`。合入事实见 [GitHub 回读原件](ci/1738-MERGED26AP.json)。

本票把 Unix 输送测试的固定睡眠改为客户端等待 EOF，之后才检查回包接收端已销毁。两秒只限制测试等待，生产回包期限仍为 10ms。代码差异仅在测试段，未改生产逻辑。

[Opus 非作者独审](1737-INDEPENDENT-REVIEW26AL.md) 对该精确候选给出 PASS。它核实了内层 `reply_rx` 先于函数参数 `stream` 析构，客户端 EOF 因而能证明回包接收端已断开。

## 报告原件

以下四份报告逐字保留。报告中的 OPEN、待独审和 CI 运行中是各自出具时的状态，后续终态以本索引及 `ci/` 原件为准。

- [任务与验收](MACOS-DEADLINE-TASK26AI.md) 记录原授权范围。
- [失败分诊](MACOS-DEADLINE-TRIAGE26AI.md) 记录旧断言缺少同步、历史 main 同败与双路检索。
- [作者交付](1737-DEADLINE-REPAIR26AI.md) 记录实现、实验、靶向运行与更正的测试筛选。
- [非作者独审](1737-INDEPENDENT-REVIEW26AL.md) 记录析构顺序、证据适用范围与非阻塞建议。

## 原始运行证据

- [受控通道实验源码](runtime/controlled-channel.rs) 是既有实验原件，未接入构建。它在 nc-remote 执行，证明 receiver 仍活时跨过截止并不保证 `try_send` 失败。
- [第一次运行日志](runtime/20261026-152857-a053ca2322.log) 记录旧臂 `REPRO_OLD_EXIT=101`、同步臂 `CONTROLLED_SYNC_PASS` 和真实 Unix 测试通过。其经济测试筛选得到零项，不计通过。
- [更正筛选后的日志](runtime/20261026-153154-a053ca2322.log) 记录一项经济截止锁和两项输送测试通过。重复运行的同名测试不累加为不同测试。

两份日志首行命令回显存在引号或变量渲染缺损，原件未改写。实际输出与可复跑实验源码分别保留。两份日志首行各有一个原有行尾空格，全文 `git diff --check` 会报这两处；其他新增文书通过检查。

## 候选 CI 与主线 CI 分开核验

候选 `a053ca2322ac9ab8d27d98e376f9c569024046bb` 已取得两份成功读数。

- [普通 CI 40286432844](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40286432844) 为 `completed/success`，见 [回读原件](ci/1737-PR-CI-SUCCESS26AP.json)。
- [macOS CI 40286381749](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40286381749) 为 `completed/success`，包含经济域与输送库测试，见 [回读原件](ci/1737-MACOS-SUCCESS26AN.json)。

2026-10-26T08:35:26Z 回读时，落地主线 `d310c46dd2710c6faf3c946e8e3e05b48f1034de` 的 [普通 CI 40292862515](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40292862515) 排队中，[macOS CI 40292862501](https://github.com/xy7365527-lang/NewChanlun/actions/runs/40292862501) 运行中。对应快照为 [普通 CI](ci/1737-MAIN-CI-SNAPSHOT26AP.json) 和 [macOS CI](ci/1737-MAIN-MACOS-SNAPSHOT26AP.json)。快照不是最终结论，关票时由主控回读当前终态。

## 关票还需主控完成收尾

本目录只归档文书，不关闭 #1737。文书候选仍需非作者审查、必要 CI 与用户 main 批准，再合入主线。主控还需核落地主线 CI、处置本票实施与归档工位，以及按 `docs/agents/delivery-discipline.md` 填写关票声明。

[主控合入登记](https://github.com/xy7365527-lang/NewChanlun/issues/1737#issuecomment-6411636647) 已明确保持该票 OPEN。

本票证据覆盖 I/O 等待原截止并断开回包通道，以及已有经济截止锁。完整 `economic-session/2` 服务端到端无新增持久回执验收仍归 #1716，不能由这次测试同步修复替代。

## 文书归档的 DevSkim 误报返修

首个归档候选的 DevSkim 扫描将五份 CI JSON 中的六个 Git 提交号误判为凭据。[精确误报登记说明](DEVSKIM-REVIEW.md) 给出逐项来源、既有完整身份绑定机制与同一原始 SARIF 的前后 gate 读数。12 份原件保持逐字内容，新候选仍需差异独审与 GitHub CI。
