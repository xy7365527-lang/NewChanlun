# #1372 独立AC3补验 r3：原30秒期限耗尽

结论：**NOT_VERIFIED**。真实有效游标Watch已返回包含修订的96..99四批，且没有Gap；原正式Client逐批同cut完整分页时耗尽原30秒总期限，最终仍保留cut95。没有完整提交，不能关闭有效跨修订续接义务。

## 冻结范围

启动HEAD `68198a6cdabbdbfeee916a00321442999c3de673`（较2317369仅新增README文档）；S二进制仍为SHA-256 `5bb7f1dfd57e1d8e6cfc1097360246ae27333776504767df4aba1b9fba9ce71b`。457份实际源码、配置、输入及产物原字节在运行前保存，前后SHA全部一致。

独立AC3臂事前明确**不注入Q故障**。D层适配器仅将原Run.phase的 `restart_query` 强制为false，没有伪造q_restart生命周期。160条原输入、500毫秒调度、op95 afterBegin SIGKILL/S epoch2恢复、原S/Q资源、正式Client/HTML校验器、31行分页、最多4个Watch批次、30秒命令总期限保持。此臂不替代R4原109代Q故障双跑。

直接复用r1未改动的准备消费者，完整load90→Watch94→Watch95。在实际head95时固定已安装cursor95，真实head到99后才发目标Watch，期间没有读取推进该cursor。r2额外准备到96的门没有沿用。

## 实际续接与失败

[原始Watch响应正文](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r3-ac3/http-consumers/revisions/commands/revision-three-commit-reconnect/http/00001-response.body)返回gap=null、observed_head99、next_cursor99和连续96..99四批；cut99包含3项withdrawals与3项replaces。公开Client随后执行原有严格分页对拍：

| 同cut分页 | 完整响应页数 |
|---|---:|
| cut96 | 28 |
| cut97 | 28 |
| cut98 | 28 |
| cut99 | 6；第7页被原期限取消 |

目标命令实际用时 **30,001.399毫秒**。第92次HTTP请求因“命令或生产Client的读取期限已取消请求”终止，捕获0字节响应。结果明确 `full_state_installed=false`、`committed_changed=false`、`retained_cursor.after_generation=95`，没有自动重试、没有扩大期限或资源，没有把暂存的前三代当成最终提交。此轮Q无故障，不能将失败归于Q SIGKILL。

- [原目标命令结果](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r3-ac3/http-consumers/revisions/commands/revision-three-commit-reconnect/RESULT.json)。
- [最后一次取消请求的完整元数据](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r3-ac3/http-consumers/revisions/commands/revision-three-commit-reconnect/http/00092-metadata.json)与同目录请求/空响应原件。
- [全部事实、HTTP及逐命令摘要](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r3-ac3/SUPPLEMENT-FACTS.json)；[原事件精确行号索引](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r3-ac3/EVIDENCE-INDEX.json)。
- [事前计划](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r3-ac3/PLAN.json)、[启动命令与实际HEAD](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r3-ac3/EXECUTION.json)、[源前后核](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r3-ac3/PROCESS-RESULT.json)。

总计256次HTTP交换（255完整、1次取消），请求及响应原字节 **27,487,903** 字节；全部记录的请求/响应/正文长度和SHA复核相符。采集工具正常退出只证明原件记录完成，业务状态仍NOT_VERIFIED。

输入驱动完成160提交和495条记录。与R4-a对照的**前494条输入、最终收据、权威切面、表及schema子域**逐JSON相同；末条生命周期仅含S故障、没有Q故障，全495条不等，不能宣称全轨迹等价。本工位只产实际补验事实，等待其他工位独立判断。

首次18879端口预检遇到主控另一项只读Q，发生在任何本轮Popen及源冻结前；原未执行计划/config和预检失败说明保留。遵照主控重新分配，实际运行18880，未接管18879服务。本次S PID 86632、Q PID 86284按事前计划保留；登记原件另存 `owned-process-records-at-completion/`，后续操作须重新核真实身份。

## 三轮材料边界

| 轨迹 | 声明Q故障 | 实际结论 |
|---|---|---|
| r1 | 原109代故障 | 真实无Gap 96..99响应后，同cut96分页被Q故障打断，保留cut95 |
| r2 AC3 | 无 | 额外准备cut96完成时head106，错过事前首修订门，未发目标Watch |
| r3 AC3 | 无 | 准备cut95与原始修订响应成立，完整逐批分页耗尽原30秒，保留cut95 |

三轮都已保留失败；当前不存在完整有效跨修订续接通过证据。本报告没有要求放宽正式判据，也没有改变用户已冻结输入或原R4故障义务。
