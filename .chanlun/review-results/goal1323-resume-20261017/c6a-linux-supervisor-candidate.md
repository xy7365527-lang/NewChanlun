# #1406 C6a Linux 外层监督器首版候选

名分：绑定 #1406/#1323 的实施与独评工作草稿。2026-10-17。

**交付的是可独立审查的监督器候选，状态仍为 BLOCKED_PREPARATION。** `CONTRACT_READY=False`、`SEMANTICS_READY=False`、`REVIEWED_SOURCE/TREE=PENDING`；没有运行资格。本次无commit/push/dispatch，无cargo、SQLite、产品服务、HTTP、真实进程信号或外盘操作。

## 基线、所有权及快照

工作树：`/Users/silencehan/Projects/NewChanlun/.sandcastle/worktrees/resume-1406-c6a`，候选基线 `b6e690ecae5723fbbcb773f15400c71d1010da87` / tree `601a3f4bd54c22358eddfbf6ad51cd268fa9de5b`。

仅新增所分派两发行文件；未改原driver/fixture/旧CI，也未改另一个代理正在独审的准备件。工作树另有既有未跟踪 `run-c6a-remote.py`、`test-run-c6a-remote.py`、`ci-c6a.yml`，它们不能随候选发行；当前dirty树当然不能通过真实源身份门。没有删除他人文件。

| 文件 | bytes | SHA256 |
|---|---:|---|
| `.github/scripts/run-1406-c6a.py` | 28144 | `1763dfce2688359eb2311c55a95b268252d4e822c7adaf84d1d4865818091071` |
| `.github/workflows/issue-1406-c6a.yml` | 1246 | `6575eb15c0fc3572188ffb61ddfdfee628c28ebc7c3ba15727ef584ac04d348d` |
| 主仓本目录 `test-c6a-linux-supervisor.py` | 11343 | `423064d408751ec90abb25c80b624731dba5a8eb21b04a8adb912805b5b03c8f` |

阅读的另一准备件快照 `run-c6a-remote.py` SHA256=`9f8a6a40a112d91982fe0093e0fdb2f39dd60c757032a16a0ee6c34dd59dca25`，18026字节。只参考其边界与接口，没有动态import或发行依赖；完整41步语义器由独立工位实现后嵌入本runner。

## 本候选已装配的控制流

- Linux `PR_SET_CHILD_SUBREAPER`，唯一收割者，要求SIGCHLD默认；`waitid(WNOWAIT)`保有直属leader身份直到最后一次组信号。身份丢失或已reap时拒绝组信号；ESRCH不再补发KILL；正常退出后只观察组消失与收割后代。
- Popen新建组；handler只记取消，不在Popen返回/OWNED登记中抛出。取消后有界归档/封口继续，但不能PASS；收据发布后handler追加取消原件并强制exit1。
- 同一t0的build1200/runtime1830/verify1920/archive2040/final2070。workflow第一步记录monotonic原点，checkout也消费这个窗口。每个阶段留TERM3/KILL3与观察余量；cleanup没有追加新时间窗。
- 构建、driver、git/来源核验、二进制/原件hash、41步验证接口、归档、最终清单均经阶段worker进程组。资源探针也独立成组并受有界监督，不让大hash/归档在主控线程无限运行。
- C7a原资源帽：启动8GiB磁盘/3GiB MemAvailable；构建RSS5GiB、运行及helper RSS2GiB；磁盘余量1GiB、MemAvailable512MiB、同文件系统增长6GiB。采样包含被测组和采样worker自身RSS；采样失败即主失败。
- 真实构建命令保留locked、s_session、jobs2、E/B两个bin（通过绝对manifest-path定位rust）；严格六键build证据、11源码与冻结输入；构建前后/运行后源码、二进制hash绑定。允许尾差恰为原两CI文件。
- driver外层观测Popen前及WNOWAIT退出时间，为语义器提供实际区间、解释器/checkout/binaries路径、前后binary hash、driver ARTIFACTS原摘要及外层完整清单摘要。
- 成功或失败都先保原件再归档；未确认回收组时保留scratch而不移动正在写的原件。普通文件逐项hash，socket特殊条目列账，symlink/其他特殊文件拒绝。最终PASS须验证、归档、清单、回收、无取消、无迟到、实际exit0共同成立。
- 原触发为push `validate/1406-c6a`。35分钟job；`--preflight` 当前exit2阻止后续运行；always上传证据。没有workflow_dispatch入口。

## 需要独审明确确认的实施细节

1. 资源采样helper活动窗口保守限制为一个0.25秒原采样周期，另在原阶段内预留回收；这不是声称恢复到了一个独立旧helper超时常量。代价是繁忙runner可能提前失败，不能自动重试或放大。所有阶段还预留父组回收位置，实际成功窗口可能略短于原最大端点。
2. RSS是采样检测，不是cgroup硬内存分配上限；与原C7a口径相同。主控只读有限16KiB收据，实际内核不可中断I/O仍不可能靠Python绝对保证消失，不能声称覆盖SIGKILL宿主或内核失联。
3. 最终清单排除根目录当前封口及其动态资源探针文件（序号阈值明确写入清单）；raw-evidence同名文件绝不排除。所有原件仍上传。最终阶段收据及最终RESULT不自引用hash；这一封包边界须独立评审接受，不应只看manifest存在。
4. 对未确认回收的组不继续使用历史PGID发信号，不删除scratch，最终失败。真实回收能力尚未运行验证。

## 实际验证

```
python3 -B .chanlun/review-results/goal1323-resume-20261017/test-c6a-linux-supervisor.py
Ran 27 tests / OK / exit 0

python3 -B .sandcastle/worktrees/resume-1406-c6a/.github/scripts/run-1406-c6a.py --preflight
BLOCKED_PREPARATION / CONTRACT_READY=false / source=PENDING / semantics_ready=false / exit 2
```

日志：本目录 `c6a-linux-supervisor-tests.log`、`c6a-linux-supervisor-preflight.json`。两份Python AST解析通过，三份文件均无行尾空白；未执行产品Python模块、Linux /proc资源探针或真实guardian。

27项均纯替身/临时普通文件：资源逐项越界拒绝；WNOWAIT而非reap；已reap/身份丢失/ESRCH后无真实信号；强制清理在最后信号前保leader；TERM/KILL截止；正常路径无信号；取消发生在spawn、退出、封口后的失败；观察旧样本需退出后重采；活后代拒绝；原绝对窗口不重开；迟到success拒绝；重复JSON/nonfinite拒绝；收据不覆盖；符号链接拒绝；缺语义验证、缺准入时不启动。

## 未实现/未验项

- `verify_runtime(...)` 故意抛PENDING。独立41步工位正在做transport/oracle/HTTP/startup/H3与进程账验证；没有它就不能把真实driver回执当成功。
- 原CI v3审包和原具名反例未恢复；新27测试不代销旧反例。
- 没有独立Linux/Python安全评审结论，尚无真实Linux进程/资源/取消/归档或远端运行证据。
- 源码资格仍PENDING，必须待最终语义嵌入与独评完成后绑定精确SHA/tree和仅两文件尾差；当前工作树还有其他代理准备件未跟踪，需最终只选两发行文件，不能把dirty快照当clean候选。

本阶段不关闭#1406、#1394、父TB、SPEC或整图。
