# #1394 离线验收工具

本目录是 #1394／SPEC #1340 的测试与控制工具，不是 S/E/B/Q 产品入口。`execution_authorized=false`，真实产品、Long、完整历史、AC1..5 和 P11 均为 `NOT_RUN`。

只使用 Python 标准库。在仓库根运行：

```sh
python3 -B s_session/tests/issue1394_acceptance/run_checks.py
python3 -O -B s_session/tests/issue1394_acceptance/run_checks.py
```

当前 Codex 沙箱禁止 `/bin/ps` 与 Unix socket bind，三项原 P0 测试会因此失败。可单独运行下面的明确子集；输出会列出全部三项排除项，不能把子集通过称为全套通过：

```sh
python3 -B s_session/tests/issue1394_acceptance/run_checks.py --offline
python3 -O -B s_session/tests/issue1394_acceptance/run_checks.py --offline
```

接口与边界：

| 文件 | 职责及不可替代的输入 |
|---|---|
| `driver.py` | P0 原语；每次 executable 观测比较冻结 command SHA；`OwnedDirectory` 持有 inode/fd；SQL/原件读取沿用同一 `ReadBudget`。 |
| `scheduler.py` | 从原准备合同逐项导出矩阵、负域与比较义务；仅接 `SimulationAdapter`。字符串 `mode` 不是运行权限。 |
| `journal.py` | `Journal.create/open → consume → finish`；持久一次消费、前驱链、跨进程锁、同一绝对截止。pending 消费不自动重试；重新打开不扩大帽。 |
| `control.py` | `ControlBridge.start_toy/finish_toy` 接 P0 和日记；必须具名 `UNIT_TEST_ONLY`。`perform` 明确阻断产品阶段。`seal` 仅在显式拥有目录和冻结独立封口预算时写控制原件。 |
| `evidence.py` | 逐出现序号比较原请求/原回复、有序全集、规范化持久快照、同 cut、恢复提交集合。返回 `ORIGINALS_VERIFIED`，不等于财务或结构语义通过。真实消息键映射与错误回复解码仍待正式 adapter。 |
| `financial_oracle.py` | 从冻结资金原件、初始账、完整操作日程独算锁、最坏量、共同责任和逐提交中间态。`budget` 必传；不得从 actual 倒推初始账或操作全集。结果仅为财务不变量核验。 |
| `oracle.py` | 保留金额/哈希工具；真实读库入口要求独立冻结核账原件及共享预算，转入新的多决定核验器。没有可直接执行的产品 CLI。 |
| `closure.py` | 接 `ContentRef`、实际收到的有序 `Node` 原件、独立原库 reader 和显式预算。按类型上下文遍历，核节点集合及原字节严格相等。无缓存豁免；未经位置/出生表接线的模板一律拒绝。 |
| `supervisor.py` | 可置于外部进程的 `monitor` 循环与 `SupervisorCore`；注入真实 Probe/传输前只具离线控制效力。注册后代、采样、失联、取消、身份复核和回收不授予 OS 硬隔离。 |

`fixtures/PREPARATION-CONTRACT.json` 为指定 ce7198… 原字节。其中的 null 和历史提案不能当运行配置。测试中的 toy 现金数值、账、输入、进程身份和资源值仅属于各自临时夹具，不能产正式 Voice 或财务验收成功。

本次来源、报告及检查原件见 `.chanlun/review-results/issue1394-acceptance-integration-20261018/`。这两处新增文件均留为未提交差异，供管理者收包及独立审查。

2026-10-18 同票返修 v2：正式独审01的六项修复、固定反例与本轮原输出见
`.chanlun/review-results/issue1394-acceptance-revision2-20261018/REPORT.md`。
上段及旧报告描述旧包交付时状态；历史测量未改，不能当本轮字节的测试成绩。

新增 `captured_cut.py` 只消费已捕获原字节，核固定 Q 分页、financial 全 basis
闭包和六族全集，返回未认证事实；`voice_bridge.py` 消费显式生产字段映射、
实际 terminal/K/source 与独立 γ 原件，返回全部未认证候选或缺项。二者无产品启动
能力，不生成独评 APPROVE、不自动 BindVoice。完整合同、真实 wire 形状与边界见新报告目录
`CONNECTION-CONTRACT.md`；未来统一 producer 未冻结，所有运行值仍须根逐对象冻结。

只运行本轮影响面的回归（在本目录执行）：

```sh
python3 -B -m unittest test_scheduler test_scheduler_integration test_evidence test_financial_oracle test_supervisor test_journal test_control test_review01 test_capture_bridge -v
```

`test_review01` 保留独审固定输入并补成对成功/拒绝路径；`test_capture_bridge` 是明确
toy 捕获流与字段映射，不能当正式 G/terminal/K 输出或 P11。普通及 `-O` 实际成绩见本轮报告。
