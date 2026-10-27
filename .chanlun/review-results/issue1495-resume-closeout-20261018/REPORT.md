# #1495 恢复与收尾证据候选

当前交付状态：Lean实施已由PR1501合入main；2026-10-18旧WIP已逐块核实被交付版收编、外盘可恢复备份已独核，两个旧工位和已合本地分支已按根具体批准实际处置。本目录是纯文档候选，尚未推送、审查或合入，不宣称该报告已满足发布/关票门，也未关闭#1495。

## 原交付与本次范围

原source为9e1b22aa8b50de9097f4ce6d0247281707f8506c；纯文档证据head859886fdb7dc664f59358fc427db23122122adc0；PR1501 merge为fdc5c6a7fd2fcb6b8d436d419984c1c826b669f6。本次父origin/main为0c397d20de4076dbb6f7caa62738fc04cf72322f。

原已入Git报告、独审、验证与日志全部保留在`.chanlun/review-results/issue1495-lean-first-break-20261015/`。本次对其SHA256SUMS列出的8项逐一重算全匹配，收据见EXISTING-DELIVERY-VERIFICATION.json；未重复Lean实施或构建。当前主线run38685059094实时确认绑定merge fdc5c6a7：fixture-drift job及lake build/三fixture检查步骤成功；整run为failure，rust-check失败，test与s-observer成功。仅复用此前已登记的Rust旧失败责任，不宣称本次独立重算其失败指纹或CI全绿。

## WIP收编、备份与清理

旧外盘工位仅一个文件188新增行。逐声明去注释/空白后24个可执行块全部包含在已合版且整序列连续包含；6个注释块中4个逐字保留，另2个在交付版收窄不实范围声明/补充同向与先破解释，不应回灌旧文。详见WIP-DECLARATION-COMPARISON.json和WIP-COMMENT-COMPARISON.json。该比较证明的是本片旧WIP被收编，不能扩展成所有历史消费者已迁移。

仓外备份`/Volumes/AgentStorage/issue1495-old-wip-backup-20261018`约176K，保存完整原件/基线/patch/恢复件；本目录只收必要摘要和方法，不复制源码快照、patch原件或缓存。BACKUP-SHA256SUMS.txt为备份原清单的逐字副本，其相对路径一律按上述仓外目录解析。10项原件在清理前后全部重核，清单自身SHA不变。原件与独立restore-check同SHA，反向git apply --check通过。

清理前最后复核：dedf11原HEAD/status/patch与备份一致，未追踪0，忽略1144项全formal/.lake/；两分支无delivery以外未交付commit。当前权限可见lsof的cwd/打开文件与ps完整路径/分支参数检查命中0。原预检没有被当长期写锁。

根明确批准后只执行三条定向Git命令：移除`/Volumes/AgentStorage/sc-wt/1495`、移除缺失的`/private/tmp/goal1323-native-20261012/wt-1495-recovery`注册、普通branch -d删除`codex/1495-lean-segment-turn-20261014`和`codex/1495-lean-recovery-20261015`。三步exit0，worktree前后差集恰两目标；两本地branch已不存在，备份原件和提交对象仍在。未全局prune、未-D、未推送、未碰其他工位。执行原件见CLEANUP-EXECUTION.json，先后方法见相邻MD报告。

## 证据顺序与限制

RESUME-READINESS.md是处置前历史快照，其中“尚未清理”的状态由后续CLEANUP-EXECUTION.md/JSON推进；原快照保持原字节，不能把它单独当最终状态。CLEANUP-PREFLIGHT记录只读检查，CLEANUP-EXECUTION记录已执行事实。以上原件涉及`/tmp`的路径是当时工位来源，入Git副本由本目录同名大写文件承接。

原交付只证明声明输入域中的FirstBreakConfirmsUp/Down、TurnWindowState及等价引理，并保原严格分型；未证明raw到有序FeatureElem连接，未迁历史消费者。Rust1493、完整1323六目标、资金/隔离/恢复/规模验收不在本收尾内。K4及前端边界不变。没有把备份清理当功能完成证明。

## 本候选验证

纯文档写集限定本目录。git diff --check和文本凭据模式扫描通过；无cargo/rustc/lake/产品或新CI。实现与原158 jobs/三fixture验证证据复用原已审入Git交付；本候选应作为纯文档审查，不凭本地commit声称已发布。resolution仅为RESOLUTION-DRAFT.md，须根审查并实际发布本证据后再确认关票门6，不预写已合本目录或已关票。
