# #1406 C6a 外层监督器返修 v2

依据 `c6a-supervisor-review.md` 的1 HIGH、1 MEDIUM及主控明确批准的上传尾部预算修复。只改独占工作树两发行文件与本票纯替身测试；未嵌入语义器，未改他人准备件。没有commit/push/dispatch、cargo、产品/HTTP、真实进程信号或外盘操作。

## 返修快照

工作树 `/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`。

| 文件 | bytes | SHA256 |
|---|---:|---|
| `.github/scripts/run-1406-c6a.py` | 29632 | `7f2318dab28d447fb3feea5d4f3e0c99cc4853a3f1c67cad141be346732add5b` |
| `.github/workflows/issue-1406-c6a.yml` | 1493 | `73eeb3d1361620afb8e362474408ad444b1a5d93c2b52195c9b7ad0cd7a14b28` |
| 本报告目录 `test-c6a-linux-supervisor.py` | 16026 | `131a8d091bc3dcb0feb0617ab7bd87f7ed80f7690a4c8d08339a1928cfbce08f` |

## HIGH：失败原件从首次创建就在上传面

driver的实际`--output`现为 `runner.temp/1406-c6a-artifacts/raw-evidence`；随机scratch只存编译target。归档helper要求runtime与raw路径相同，不再移动原件。

未确认组回收时：保留原件原路径、不移动、不哈希、不生成完整清单；RESULT明确 `FAIL_C6A`、`raw_evidence_state=UNSEALED_LIVE_OR_UNKNOWN`，并保存精确runtime路径/上传相对路径。所有状态均含 `delivery=NOT_VERIFIED_REMOTE_UPLOAD`。上传中的原件可能仍变化，不能据此说已完整封存；其保全价值是供失败排查。

新增测试从真实外层argv取output（没有硬编码旧scratch路径），放一份失败原件并模拟未回收组；核exit1、原件确实在唯一上传目录、未生成tar或FILES.json、UNSEALED与未交付状态。所有进程、信号、时钟、资源和Linux激活均为替身。

## MEDIUM：排除先于stat，限定当前封口命名空间

`regular_files`现在先依据相对名字判排除，再调用DirEntry.stat。`finalization_file`只展开 `save()` 的精确临时名字 `.NAME.tmp` 后核当前封口序号阈值。非本轮序号、一般隐藏文件以及raw-evidence中的任何同名文件仍不排除；没有捕获并吞掉FileNotFoundError。

新增测试模拟已经从目录枚举返回、但在stat之前消失的`.probe-00010.json.tmp`，其stat函数若被调用便失败；实测排除在前、stat未被调用。另核raw-evidence中同名原件仍入清单、真正原件缺失仍抛错。

## 上传阶段独立有界，产品端点不变

按主控批准，为远端证据传输单独设**15分钟**step上限；job总窗由35改50分钟，真实执行step仍35分钟。脚本同t0的1200/1830/1920/2040/2070端点和所有资源值均未改变；上传阶段没有任何启动/重启产品或helper的动作。缺失artifact由warn改error。

15分钟是本次明确的上传工程预算，不冒称旧合同恢复值，也不承诺任何特定网络吞吐必达。执行成功、本机RESULT、archive、FILES.SEAL或上传step存在均不证明交付；最终须独立核实际upload step success、远端artifact可取及清单内容。未回收失败中的UNSEALED原件即使上传成功也不成为成功验收。

## 实际验证与剩余门

- 纯替身测试 **32项通过，exit0**。日志 `c6a-linux-supervisor-v2-tests.log`。在原27项上新增5项，覆盖两条独审反例和尾部预算不改执行端点。
- `--preflight` 实际 **exit2**，`CONTRACT_READY=false`、`SEMANTICS_READY=false`、来源PENDING。输出 `c6a-linux-supervisor-v2-preflight.json`。
- 两份Python AST解析通过、两发行文件与测试均无行尾空白。

原独审脚本仍保持原件，没有改成作者自己的通过报告；其旧反例断言本来针对旧路径/旧缺陷，复核应由原审者按修正后argv与原问题检查。请原审者复核此精确快照。

仍欠独立返修复核、完整41步语义器及原容量门问题、最终源尾差资格、真实Linux/远端运行和实际上传交付证据。本返修不触发运行、不关闭任何票。
