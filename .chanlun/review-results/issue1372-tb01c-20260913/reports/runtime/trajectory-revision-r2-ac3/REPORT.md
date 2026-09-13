# #1372 独立AC3补验 r2：准备阶段越窗

结论：**NOT_VERIFIED**。该臂事前明确不注入Q故障，保持160输入/500毫秒、S op95 afterBegin故障及原资源。它没有发起目标修订续接；不得作为有效续接成功证据，也不能替代R4故障双跑。

启动绑定HEAD `2317369c3de2e3d9e8d90a545dd32d0e759d2b8b`，457份源码/配置/输入/产物前后SHA未变。运行期间root新增README文档提交不在运行绑定集合内；按真实启动HEAD报告。

公开Client依次完整安装90→94→95→96。最后一次Watch95→96耗时5.277秒，安装时真实head已106。由于冻结计划要求在首修订cut99发生前完成准备，该门实际失败，完整保留错误，未放宽门或补发目标Watch。

HTTP交换 193 次，完整 193 次，全部请求/响应原字节 20334111 字节，附件SHA校验错误 0 项。具体原件、逐命令结果与实际游标见 [SUPPLEMENT-FACTS.json](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r2-ac3/SUPPLEMENT-FACTS.json) 与 [消费者原结果](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/issue1372-tb01c-preparation/runtime/trajectory-revision-r2-ac3/http-consumers/RESULT.json)。

原输入驱动生成495条记录，输入全部160次提交，运行进程与采集进程均正常退出；进程退出不等于场景通过。与R4-a逐JSON对照的**前494条输入/最终收据/权威切面/全表/schema子域**相等：True。末条生命周期仅含S重启，明确不含Q重启；全495条不等。不能用本臂宣称原R4全轨迹等价。

本臂与r1的准备差别是额外追到96；实测该额外完整分页消耗了首修订窗口。后续若重新尝试，须另建计划和目录，保留r1已实际在head95完成的cut95准备入口，同时明确不注入Q故障；不能修改本臂原件追认成功。

本次专属18878端口的S/Q按事前计划保留，登记原字节另存 `owned-process-records-at-completion/`。其他实例未接管，root源/原collector/R4与r1原件未改。此报告仅产出实际事实，最终判定由其他工位完成。
