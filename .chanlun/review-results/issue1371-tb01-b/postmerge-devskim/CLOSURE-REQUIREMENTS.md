# #1371 关票入仓与工位收尾要求

结论：现行纪律要求本票报告实体在关票时已入 Git；它没有授权把任何新文档提交自动合入 main。两项要求分别执行。固定 `256cd822e0d872f1fb65d433ca0b15b01fa621a2` 已包含产品与完整独评交付包，但最新 R14 扫描分诊、修绿可行性和收尾预检尚为 D-only，需要一个有限的纯文档尾包。冻结报告不回写，产品不重测。

根刚提供：PR #1449 已按固定 head 条件批准合入 `d2219e8e014a5d55c091f8eda62ffb9dd8154953`，tree `6fd745ce161d8c4e4cc341483435bef24b810831` 与256相同；主线 CI `34714772462`、DevSkim `34714772442` 当时运行中。本子任务未重复轮询，不能据此预写最终检查通过或工位清理完成。

## 直接条款与当前未满足项

| 要求 | 条款 | 本轮结论 |
|---|---|---|
| 报告入Git、相对路径引用 | DD:16 | 本票报告实体不能只留D绝对路径；最新分诊/可行性/工位报告和本预检需归入有限尾包。 |
| 实际交付进入main；声明在当时为真 | DD:11–13 | 256合入由根提供新事实；尾包仍未生成/入main。 |
| 新提交合入批准与入仓分开 | AG:154、164 | 明文是“开工授权不替代…合入 main…明确批准门”。当前用户授权仅覆盖刚执行的固定256例外；先准备具体尾包，再处理新head的明确main门，不重新问256。 |
| 逐提交可编译性 | DD:14、21 | 既有仓内清单覆盖24项；256第25项纯报告豁免仍在FD仓外回执。尾包将其原件入仓，新增文档及实际merge提交逐项如实声明。 |
| 评审面等于关票面 | DD:17、19–21 | 纯文档/注释尾提交明文豁免追加代码评审；只有新增代码才须补评审，不因为归档报告重测产品。 |
| 对应main CI与红灯去向 | DD:74–80 | 给实际main SHA/run/结论；红灯照实写红及承接去向，不改ignore、范围或分支以制造绿。 |
| 工位清理与在制品处置 | DD:15；GC:24–25 | 14工位仍只有预检，9份指定在制品已逐字保全。删除前现场再核；OPEN阶段不提前按残骸广删。 |

DD 指《交付纪律》，GC 指《世代宪法》，AG 指现行 AGENTS.md，文件及SHA见文末和JSON。DD/GC工作区字节已与固定256 Git对象核为一致。没有从技能或解释另造批准门。

## 最小可执行尾包

建议新目录 `.chanlun/review-results/issue1371-tb01-b/closure-tail/`，只做一个具名报告归档提交。已有两包不改：`native-codex-r7-r12/` 和 `native-codex-r13-r14/`。本轮从固定Git树确认其报告实体存在、索引分别423/213条；没有重新解码全附件或重审产品。6189→256只23份报告文件，base→256共25提交。

尾包实体清单（保留原字节；大JSON可按既有xz+base64方式归入内容寻址附件）：

- `work/PR1449-DEVSKIM-R14-TRIAGE.md`
- `work/PR1449-DEVSKIM-R14-TRIAGE.json`
- `work/DEVSKIM-REPAIR-FEASIBILITY.md`
- `work/DEVSKIM-REPAIR-FEASIBILITY.json`
- `R14-FINAL-DELIVERY.json`
- `work/POSTMERGE-WORKSTATION-PREFLIGHT.md`
- `work/POSTMERGE-WORKSTATION-PREFLIGHT.json`
- `work/POSTMERGE-WORKSTATION-R14-PREFLIGHT.json`
- `work/POSTMERGE-WORKSTATION-PRESERVATION/MANIFEST.json`
- `work/POSTMERGE-CLOSURE-REQUIREMENTS.md`
- `work/POSTMERGE-CLOSURE-REQUIREMENTS.json`

另写一份有限收尾说明：固定来源256、条件批准事实、真实DevSkim FAIL、9,232个有反证键与1个旧needs_review、实际安全缺陷总数未知、main/CI和清理事实的截点。已有R7/R13 FAIL、R6 INCOMPLETE、R14 PREMERGE措辞原样保留；不能把当前结论写回旧原件。R14-FINAL-DELIVERY中的历史main_merged=false同样不改。

最新分诊JSON为65,909,936字节，可压缩原件入附件；索引须提供仓内路径/JSON pointer、原文件名、原长度/SHA和编码，解码后核原字节，关票评论引用仓内实体路径。相关run/checkout/规则来源记录保留必要小附件或不可变来源链接。巨大SARIF/zip、数据库、二进制、全D备份和会话导出不因报告入仓要求一并收编；明确哪些原始副本仍留D。9份工位保全副本留D，其中旧R6早已在既有入仓附件可恢复；保全manifest和处置事实进入尾包，不把8份旧roster修改混回冻结产品。

先核尾包可恢复、diff只有报告、旧报告与产品hash未变，然后固定新提交供main批准。**这是入仓义务驱动的具体准备，不是新提交的合入授权。**用户对256的条件批准已经执行，不需重复请示该版本；新文档head的合入和扫描例外不能伪装成旧SHA。

为阻断“扫描报告本身→新分诊报告→再扫描”的递归，本次报告结论绑定固定256及具名历史run，尾包有明确截点。其后一次CI的机器回执是外部CI事实，最终resolution用实际SHA/run链接照实记录；不必把每一条新日志再包装成独立研究报告并产生下一提交。新代码、真实新缺陷或来源改变仍须按实际影响处理，不能据此关闭扫描。尾包不自引尚不存在的commit hash；最后resolution填写实际hash即可。

## 14 工位保护与清理核验

P14快照列14条，全部HEAD是256祖先；5条干净、9条仅原登记/旧R6未提交面。今天独立重算9份保全原件长度/SHA，全部与manifest一致；没有重查14树当前状态，执行人必须在删除前现场核。

| 精确工位 | 快照HEAD | 快照状态 |
|---|---|---|
| `/Users/silencehan/Projects/NewChanlun-1323-spec-publication/.sandcastle/worktrees/codex-1323-tb01-b-1371` | `4922543038fb` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-codex-r12` | `750f933bddff` | 干净 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-acceptance` | `8c868f968c5f` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r10-acceptance` | `3ddb5c2b7f9f` | 干净 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r11-acceptance` | `4922543038fb` | 干净 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r2-acceptance` | `0298cf429804` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r3-acceptance` | `a9acf50ac3a7` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r4-acceptance` | `58dab231e221` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r5-acceptance` | `8b726a57f989` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r6-acceptance` | `6eeed76e3e12` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r7-acceptance` | `4a3da3787bdd` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r8-acceptance` | `65fb5e72bbee` | 指定旧roster/旧R6；已保全，现场仍需核 |
| `/Users/silencehan/Projects/NewChanlun-1371-native-r9-acceptance` | `667ce698eacd` | 干净 |
| `/Users/silencehan/Projects/NewChanlun-1371-codex-r13` | `256cd822e0d8` | 干净 |

只处理三条具名本地分支：`codex/1323-tb01-b-1371`、`codex/1371-codex-r12`、`codex/1371-codex-r13`；先解除关联worktree，核没有未合入提交后才删除分支。远端分支由根发布流程核，不连带删除其他ref。

保护用户主仓 `/Users/silencehan/Projects/NewChanlun` 的全部脏改；保护父树 `/Users/silencehan/Projects/NewChanlun-1323-spec-publication`（只能处置其内原B精确子树）、整个D及14条之外所有工位/分支。P13曾列66个外部工位和主仓21处跟踪改动/333条未跟踪记录，这是历史快照，不能当作现时计数或清理依据。

现场核验最小集：精确注册路径、HEAD及对实际交付main的祖先关系；跟踪/未跟踪/ignored面；9份保全映射；本票PID/argv/cwd和关联端口；是否被其他活动票/会话接管。新文件、新commit或新进程不能被旧预检吞掉，先剔出并处置。禁止广泛force/clean/reset、按名称杀进程或递归删父目录。清后比对worktree/ref清单及14条结果，保护范围不得变化。

两条纪律有顺序约束：关前先处置在制品、核已交付且可清，关票状态转换后按“票已关+干净+无未合入commit”清精确残骸，再发表“工位已删”的完成声明。把它作为同一受控收尾，不把未来动作预写已成；任何清理失败不得维持“收尾完成”，必要时复开/保留待处置状态。月度归档是关票之后的周期动作（GC:52），不能代替关票时先入Git，也不授权现在删D。

## 来源与范围

- **DD** `/Users/silencehan/Projects/NewChanlun/docs/agents/delivery-discipline.md`；定位 `11–21;74–82`；SHA256 `efa40109c11f4d57f034fd3919c4cfc90894c93c54568fe92743f9a9a4d9be13`。
- **GC** `/Users/silencehan/Projects/NewChanlun/docs/agents/generation-constitution.md`；定位 `20–25;36–37;50–52`；SHA256 `6871ddb943cbc523c2c5a228bb1046adf9e27072b7c1ba929c7d8c683e23021e`。
- **AG** `/Users/silencehan/Projects/NewChanlun/AGENTS.md`；定位 `154;164`；SHA256 `f185e3eb6d6750d6168b4e5268282ac9b2dd0cd6514d8fb045151c10b1db2957`。
- **P13** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/POSTMERGE-WORKSTATION-PREFLIGHT.md`；定位 `全文；旧13工位快照`；SHA256 `ad07f01fc1ee6f96e08de98b36d9b4c8784a7e349c5c77a5dc22dc496e056ebf`。
- **P13J** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/POSTMERGE-WORKSTATION-PREFLIGHT.json`；定位 `/worktrees;/other_worktrees_outside_scope`；SHA256 `70ccf26ef5be755b4f5d47aecee0e9a22fcd93902f1fef1a44b77f4576dd1937`。
- **P14** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/POSTMERGE-WORKSTATION-R14-PREFLIGHT.json`；定位 `/delivery_head;/worktrees/0..13;/protected`；SHA256 `ef9c4ed1a9592726bfee6e492dd0dbc85e7df53f09b64929018e398b4c7f0e2f`。
- **PM** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/POSTMERGE-WORKSTATION-PRESERVATION/MANIFEST.json`；定位 `/entries/0..8;/old_r6_delivery_equivalence`；SHA256 `3667deee6cf33c676737f595d736df3b1da716998c8af111a3e1452d388a31fd`。
- **DS** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-DEVSKIM-R14-TRIAGE.md`；定位 `全文；固定256分诊结论及限制`；SHA256 `490907cd28637bb8125c98fc8a3f206fd1f1bbbb6bd267243acace5574b14b26`。
- **DSJ** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-DEVSKIM-R14-TRIAGE.json`；定位 `/source;/counts;/coverage;/risk_and_merge_effect`；SHA256 `78e2b5e6295530fb41ed0d6918785346221782d395f513eecb097fe5794194c3`。
- **F** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/DEVSKIM-REPAIR-FEASIBILITY.md`；定位 `全文；不可范围内真实修绿的限定命题`；SHA256 `4ecf4612a3d0d975f4b7b955f60388edb8273a2936ab70212e75c56501fc834a`。
- **FJ** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/DEVSKIM-REPAIR-FEASIBILITY.json`；定位 `/head;/verdict;/minimal_impossibility_evidence;/options`；SHA256 `c47e6e23d855fdd06efd703cc1140a6a1d8737a458c402f8badde34df32bf4ce`。
- **FD** `/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/R14-FINAL-DELIVERY.json`；定位 `/final_head;/kind;/total_commits;/product_identical`；SHA256 `0d04c0d3abde60ffaff4aa2ff0ddf3ca9581be8a01709765e239779fa0ad4158`。
- **GREADME** `.chanlun/review-results/issue1371-tb01-b/native-codex-r13-r14/README.md`；定位 `固定256 Git对象`；SHA256 `ab94751a3743e836ecc578f664dc2bf181f669e4465e45aa090a69ce3502a333`。
- **GROOT** `.chanlun/review-results/issue1371-tb01-b/native-codex-r13-r14/ROOT-ACCEPTANCE.md`；定位 `固定256 Git对象`；SHA256 `3b5b3ec0b64595c8cf907cf2350403519a1798910d37d720f7a842ab354e726d`。
- **GDECL** `.chanlun/review-results/issue1371-tb01-b/native-codex-r13-r14/COMMIT-DECLARATIONS.json`；定位 `固定256 Git对象`；SHA256 `c897b7c6438899583bc95aa1d61db7463ca0d45f22d2a4f3e5d93c70c9d75255`。

只读纪律/Git对象/既有报告、纯解析与9份保全hash核对；仅写本MD/JSON。未改仓库/Git/GH，未清工位、测试产品、跑服务、启动Prime或研究C。#1371收尾不自动关闭#1372、#1359、#1323或#1448。
