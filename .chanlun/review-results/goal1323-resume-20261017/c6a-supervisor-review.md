# C6a Linux 监督器只读独审

名分：#1406/#1323 工作草稿，2026-10-17。审查对象仅 `resume-1406-c6a` 的两发行文件。结论 **Block**：失败原件上传缺口需修；另有最终清单竞态。语义器 PENDING 是已声明未完项，本报告不把它重复计成新缺陷，也不签完整运行 PASS。

## 固定审查快照

| 文件 | SHA256 |
|---|---|
| `.github/scripts/run-1406-c6a.py` | `1763dfce2688359eb2311c55a95b268252d4e822c7adaf84d1d4865818091071` |
| `.github/workflows/issue-1406-c6a.yml` | `6575eb15c0fc3572188ffb61ddfdfee628c28ebc7c3ba15727ef584ac04d348d` |

依据：已读根 CLAUDE/AGENTS、Serena manual、本目录 `c6a-contract-recovery-v2.md` 与 `c6a-linux-supervisor-candidate.md`。`git diff -- '*.py'` 无输出，因为发行件是未跟踪新文件，故按上述精确路径全文审查。未修改发行源码或 workflow。

## 发现

[HIGH] 回收未确认时失败原件不在唯一上传范围

File: `.github/scripts/run-1406-c6a.py:491`；`.github/workflows/issue-1406-c6a.yml:42`

Issue: runtime 创建于随机 scratch，只有所有组回收完成才 rename 到 artifacts/raw-evidence。任一驱动组或资源 helper 回收失败，会在 archive 与 seal 两处提前拒绝；RESULT 只记 scratch 路径。workflow 仅上传 `runner.temp/1406-c6a-artifacts/`，没有该 scratch/runtime。远端 runner 销毁后失败原件无法回查。保留本机目录不等于远端保全原件。

Counterexample: 本目录 `test-c6a-supervisor-review.py::test_unreaped_runtime_originals_outside_only_uploaded_directory` 将进程/时钟/资源全替换，构造一份 runtime 原件并模拟未回收组。真实 `real_run` 返回1，RESULT记录未回收；原件仍在 scratch，但上传目录下没有该文件。没有真实进程或信号。

Fix: 推荐从创建时就将 runtime 放在受上传覆盖的 artifacts/raw-evidence（同FS），scratch只存构建 target。未回收时保持 FAIL，保留原件不移动、不冒称完整封存，并明确“仍可能写入/未封存”。也可在失败路径输出绑定精确的 runtime 路径供 workflow 追加上传，避免上传整个构建 target；须明确仍可能写入，不以副本/哈希完整性签 PASS。不要在原端点后追加无限归档窗。

[MEDIUM] 最终清单未排除资源探针发布时的临时名

File: `.github/scripts/run-1406-c6a.py:112`、`:370`、`:423`

Issue: 最终 inventory 本身仍由资源探针监督。`save` 先写 `.probe-NNNNN.json.tmp` 再硬链接正式名并 unlink 临时名；`finalization_file` 只匹配不带前导点的 probe/request/序号名。inventory 因而会哈希这个临时文件，可能在发布者 unlink 后触发 FileNotFoundError；甚至 `regular_files` 在调用 exclude 前就对保存的 DirEntry 做 stat，也可能撞到临时名消失。正常运行可因调度时序而封口失败。

Counterexample: `test_final_probe_temporary_is_hashed_then_disappears` 建立相同临时名，确认没有被排除，并在 stat/hash 间模拟发布移除；真实 inventory 抛 FileNotFoundError。

Fix: 对明确属于当前封口及探针的临时名，在目录遍历的 stat 之前按名字排除；或者冻结最终清单的枚举集合，保证不再遍历并发写入的封口命名空间。不得广泛吞 FileNotFoundError 而漏掉真实原件。

## 远端上传窗口：仍是准入前待解决项

workflow 的35分钟是 job 总超时；脚本 final 端点为同t0的2070秒，名义最多余30秒给 upload-artifact（job启动至第一步也消耗总窗）。没有为上传预留与证据大小匹配的可靠窗口，也没有在此窗口内完成上传的证据。允许的文件系统增长为6GiB，上传路径还包含原件及压缩包；即使只按6GiB计，30秒也要求平均204.8MiB/s，远端网络没有这样的保证。

因此本地 `PASS_C6A_CI` / archive / FILES.SEAL 不可当远端交付通过。最小处理是把上传明确作为独立有界交付阶段并预留可执行的窗口，或调整 job 超时只为上传增加外层时间而不改变1200/1830/1920/2040/2070执行端点；数值由主控按授权/合同决定。最终验收必须核 upload step 成功及远端 artifact 可取，而不是只看本地RESULT。此处没有伪造远端运行反例或修改合同数值。

## 本次静态核实成立的部分

- `waitid(WNOWAIT)` 是直属 leader 退出观测；强制回收保持 leader 到最后 TERM/KILL，之后才 wait。已失身份/已reap拒绝实际组信号；`killpg(...,0)` 只是存在性探测。subreaper 启用，driver 子进程显式 `start_new_session=False`，继承外组。未发现当前相同组回收路径提前 reap 后实际 kill 的反例。
- build/runtime/verify/archive/final 均从同t0取绝对端点；主要大文件/hash/git/归档放阶段 worker。资源helper限制为0.25秒活动窗并在原期限内预留回收；父组另留回收量。采样值核原C7a资源数，包含被测组和probe RSS。此为采样约束，不是硬cgroup帽或实时调度保证。
- handler在运行时只记取消，封口后取消产生失败标记/非零退出；成功资格包含零取消、回收完成、阶段未迟到、exit0。失败允许有界归档继续但不恢复PASS。
- identity 核clean checkout、SHA/tree、祖先与尾差限于两CI文件；build证据使用driver11源集合与精确六键核验；构建后/运行后源身份、二进制hash均接入。当前PENDING门仍阻止真实启动。
- 最终清单明确排除本轮封口及动态日志，raw-evidence内同名文件不排除。这一边界有明文而非自称全文件哈希；排除部分仍必须随原件上传并由外部验收检查。

## 实际检查与局限

`python3 -B .../test-c6a-linux-supervisor.py`：27 tests，OK，0.016秒。

`python3 -B .../test-c6a-supervisor-review.py`：2项独立反例，OK，0.005秒（OK意指已复现所述缺陷，不是监督器获批）。

当前 Python/命令路径均未提供 ruff/mypy/pylint/black，因此未声称静态分析器通过。源码的多数公共函数缺类型注解、核心函数较长，建议返修时补注解以利后续静态检查；这些不替代本报告的具名控制流反例。

没有运行真实 Linux /proc 探针、进程/信号/guardian、产品HTTP、cargo、浏览器、外盘、远端GitHub或重型回放。未证明平台实测回收、全部41步语义、上传完整性。保留CONTRACT_READY=False、SEMANTICS_READY=False及源码PENDING；本报告不能关闭C6a或全图。
