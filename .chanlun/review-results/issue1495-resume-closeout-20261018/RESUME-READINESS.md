# #1495 恢复就绪核验（2026-10-18）

结论：Lean实施已交付到main，不应重做。票仍OPEN的实际尾务是对应工位在制品处置与关票留痕；当前不能合规关票。

## 本轮直接核实

- GitHub PR1501实时为MERGED，2026-10-15T05:07:10Z；head `859886fdb7dc664f59358fc427db23122122adc0`，merge `fdc5c6a7fd2fcb6b8d436d419984c1c826b669f6`。
- `git ls-remote origin refs/heads/main` → `0c397d20de4076dbb6f7caa62738fc04cf72322f`；`git merge-base --is-ancestor fdc5c6a7 0c397d20` exit0。
- 两交付Lean源在当前远端main与merge逐字相同：SegmentFeatureSeq SHA256 `a9978ce332cea564ab324114859a9d7a4baaea55c11bf924dd5e1af6b988ba01`；Claim10 SHA256 `dec380404b1bc208cec004dd80a5770c64c969b22617236ae00a188d4c8ac71d`。
- 根当前候选工作线这两文件与交付不等；不能用根旧源码反推main没交付。
- merge中 `.chanlun/review-results/issue1495-lean-first-break-20261015/SHA256SUMS` 八原件逐一核SHA全部通过；报告、独审、manifest、token扫描、VALIDATION、三日志已入git。
- 实时run38685059094绑定merge fdc5c6a7，fixture-drift成功（lake build及drift步骤均成功）；整体failure，rust default/backtest失败，test/s-observer成功。已存总图说明其旧失败指纹及#1474责任，本轮没有重跑或独立复算Rust指纹。

## 票面逐项

1. 引注订正已落两Lean。亲读正本071:26/28/30/32/42，与独审的正文归属核验一致。
2. FirstBreakConfirmsUp/Down、TurnWindowState、turnWindowStateUp/Down已落；原严格分型谓词保留。
3. firstOutsideWindow及非包含引理提供可达域；firstKindUp_iff_topFractal / firstKindDown_iff_bottomFractal显式保同一无缺口条件与相关极值非并列前提；并列由先破直接裁决。源码有真实by证明，不拿随机400组读数替证明。该结果只证声明输入域，未证raw→同向有序FeatureElem构造连接，也未迁历史消费者。
4. 已有精确source `9e1b22aa8b50de9097f4ce6d0247281707f8506c` 远端run38681567438/job116247066853，158 jobs及三fixture零差异原件；纯文档859886豁免编译。独审PASS_SCOPED_SEMANTIC_REVIEW，0未决必修；源码身份绑定一致。本轮复用，不运行lake/cargo。

## 关票实阻塞与下一切片

已读delivery-discipline关票门：第5项要求处置本票未合在制品。外盘 `/Volumes/AgentStorage/sc-wt/1495` 目前可读，基dedf11f0，`git status --short`确有一个Lean修改，188新增行，文件SHA256 `2f87dd64dc52d7b4c7babb5a3cd3dd41cba28e22346560f718013edc05d8647f`。其命名def/theorem/inductive符号无merge独有之外的额外名称，merge另有10个新符号；这仅为分诊，不能据此宣称旧WIP逐义已收编。

注册工位还包括 `/private/tmp/goal1323-native-20261012/wt-1495-recovery`（gitdir显示prunable/nonexistent）及外盘旧工位，两分支均保留。没有查到本轮1495在飞写者；历史Sandcastle记录只作历史，不作运行证明。认领仍xy7365527-lang，无需重新实施或另开票。

最小下一切片：只读保存旧WIP完整patch、未追踪文件清单、分支未合commit清单及SHA到仓外；对比9e1b22源码和旧WIP逐块，确认无遗漏独立成果；由根决定备份归档后处置两工位/分支，再按关票门发布resolution。无需改产品/定义、无需重建158项、无需重跑全图历史。当前可写集仅归档收据及活票工作草稿/roster，产品写集为空。破坏性删除不在本轮执行。

## 给根的resolution草稿（目前不可发布成关票声明）

交付为 `9e1b22aa8b50de9097f4ce6d0247281707f8506c`（Lean源码，可编译，run38681567438），`859886fdb7dc664f59358fc427db23122122adc0`（纯文档证据，编译豁免），经PR1501合入main `fdc5c6a7fd2fcb6b8d436d419984c1c826b669f6`。定义、三态、首个出窗可达域和上下等价证明及引注订正已验；严格分型谓词未改。报告见 `.chanlun/review-results/issue1495-lean-first-break-20261015/REPORT.md`，独审同目录INDEPENDENT-REVIEW.md。main CI38685059094的Lean/fixture通过，整条CI为红，历史Rust失败与DevSkim责任维持既有总图登记，不宣称全绿。历史消费者迁移、Rust1493、raw连接及1323整体验收不包含在本票交付。

仅在旧WIP已逐块核与归档、对应工位已按纪律处置、根复核最新main/CI后追加真实关票声明；不得预写“已清理”。

父图索引建议：`[#1495 Lean第一种情况](https://github.com/xy7365527-lang/NewChanlun/issues/1495)：PR1501已合main，精确源158 jobs/三fixture及独审证据完整；2026-10-18实时main仍保两源字节。票保持OPEN：外盘旧WIP188行待逐块归档处置；不重复Lean实施，不代销#1493或全图。`

## 搜索与限界

图检索tmp-kimi-nest-mainline只命中旧BoundaryNoInclusion，未覆盖新定义，已回退精确Git对象；XERJ19200既有nc1320src-docs搜索1495命中0，未重装/新增索引。先核roster、workers日志、worktree注册、git历史及PR实际状态，定位既有成果，未新开票。未改产品/定义、未构建、未push/关票/删除。

记忆快速检索只用于定位旧状态风险：MEMORY.md:136/145；本报告关于实际交付全部以本轮Git/PR/原件核验为准，旧记忆“历史验证阻塞”未采用作当前结论。

## 追加：旧WIP逐块核验与可恢复备份

仓外备份：`/Volumes/AgentStorage/issue1495-old-wip-backup-20261018/`。HEAD/status、binary diff、原完整文件、Git基线完整文件、已合完整文件、逐声明比较、注释比较、MANIFEST及SHA256SUMS均已保存。未跟踪文件0；忽略文件1144件全部位于formal/.lake/，属于再生产缓存，未纳备份。旧分支HEAD为dedf11f047，`fdc5c6a7..HEAD`未交付commit集合为空。

恢复实证：在独立restore-check目录放基线文件，`git apply --check` exit0、`git apply` exit0，恢复文件逐字等旧WIP原件。操作后原工位status及原文件字节再次核一致，没有reset/clean/remove/delete。

188新增行是单一hunk：24个可执行声明/instance/example逐块去注释及空白后，全部逐字包含在已合§6.1中，且整个旧可执行序列连续包含；无独有未交付代码。6个注释块中4个去空白逐字保留，第1块仅将“严格谓词只服务第二种”这一已被独审指出的不实范围声明收窄并声明历史消费者未迁；第5块增加同向笔/时间顺序及同时越两端先杀候选说明。两项都是已审合入版的订正/补充，旧文不应回灌。无无关新工作。具体24块及6注释结果见仓外JSON。

下一处置方案供根审：接受旧188行成果已全被交付版收编，保留仓外可恢复备份；核无进程持有该工位且无其他owner写入后，按既有授权处置旧工位和旧分支，另核recovery工位注册残条及分支未交付commit后清理；再以真实已处置事实补关票声明。当前关票门5仍缺“实际处置工位/分支”，安全备份与收编核验已齐；备份本身不等于工位已删除。本代理不执行删除、不关票、不push、不重复Lean构建。
