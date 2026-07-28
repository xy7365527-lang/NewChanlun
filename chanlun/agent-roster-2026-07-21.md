| [5.6]issue112-code-review | sonnet壳→codex GPT-5.6 Sol | 双轴审查 issue#112 进场门多级投影 commit，产出 chanlun/review-results/code-review-issue112-20260721.md | 派发中 |
| [codex]#479-wave1-5a5b-卡重建 | kimi编排→codex exec（claude 周额度尽，降级） | 据幸存纲要重建 wave1-plan5a/5b 实装卡（原卡佚失，#69 前置），工位 /tmp/wt-69 分支 ticket-69，落 chanlun/review-results/ | 已交付（e1e3aa7037，#479 已关，待 #69 实装） |
| [codex]#69-5a-游标驻留实装 | kimi编排→codex exec（claude 周额度尽，降级，编排者 2026-07-27 拍板） | #69 段二：trend_confirm per-pair 游标驻留（卡 wave1-plan5a-implementation-card-20260719.md），工位 /tmp/wt-69 分支 ticket-69，验收=测试全绿+250k/1M diff=0+SHADOW 0+在线计数一致 | 已交付（7c4df3e118，kimi 验收+提交；基线红 3 条在册 #446、1 条新登记 #491，零新增红） |
| [codex]#69-5b-pan-memo实装 | kimi编排→codex exec（claude 周额度尽，降级） | #69 段三：p123 run 语境细粒度 pan memo（卡 wave1-plan5b-implementation-card-20260719.md，R1/R3），工位 /tmp/wt-69 分支 ticket-69（基 7c4df3e118），提交门=零新增红+新测试全过 | 派发中 |
| [codex]#69-5b-pan-memo实装 | kimi编排→codex exec（claude 周额度尽，降级） | #69 段三：p123 run 语境细粒度 pan memo（卡 wave1-plan5b-implementation-card-20260719.md，R1/R3），工位 /tmp/wt-69 分支 ticket-69（基 7c4df3e118），提交门=零新增红+新测试全过 | 已交付（e8d5a47f06 自提交；kimi 抽验：零新增红、250k/1M cmp=0、SHADOW 27037 checks 0 mismatch、在线计数一致） |
| [codex]#494-影子评审-#69 | kimi编排→codex exec（新上下文终局门） | #69 5a/5b 实装影子评审（两轴+证据独立复核+提交纪律），工位 /tmp/wt-69，落 chanlun/review-results/shadow-494-issue69-20260727.md | 派发中 |
| [codex]#494-影子评审-#69 | kimi编排→codex exec（新上下文终局门） | #69 5a/5b 实装影子评审（两轴+证据独立复核+提交纪律），工位 /tmp/wt-69，落 chanlun/review-results/shadow-494-issue69-20260727.md | 已收口（FAIL 五条翻转条件 → 编排者四项裁定 + 封印 diff=0，#494 已关） |
| [codex]#426/#427-归属拆分补交 | kimi编排→codex exec（编排者 2026-07-27 拍板） | worktree 两未提交实装（#426 已关码未交纪律事件 + #427 在飞）归属拆分、cargo 验证、一票一提交簿记补交，工位 /tmp/kimi-nest-mainline | 派发中 |
| [codex]#421-续跑收口 | kimi编排→codex exec（编排者 2026-07-27 拍板 (a)） | SPEC 活假设账本接生产：从停滞第 3 切片续跑至收口（含 #426/#427 转呈验收清单 + 订单流字节不变对拍 + 自查留痕），工位 /tmp/kimi-nest-mainline | 派发中 |
| [codex]#421-返工三HIGH | kimi编排→codex exec（双影子 FAIL 后，编排者 2026-07-28 拍板） | P-H1 身份跨 prefix 活 / P-H2 审计接缝可达+分母 / P-H3 US15 复用 dirty/cache + T8 矛盾，工位 /tmp/kimi-nest-mainline | 派发中 |
| [codex]#421-返工三HIGH | kimi编排→codex exec（双影子 FAIL 后，编排者 2026-07-28 拍板） | P-H1 身份跨 prefix 活 / P-H2 审计接缝可达+分母 / P-H3 US15 复用 dirty/cache + T8 矛盾，工位 /tmp/kimi-nest-mainline | 已交付（f6639a2a1b；kimi 抽验：1992/1 零新增红、m8 SHA 全同、229/229 正寿命、ForceOvertake=1 带证据）；#429/#430 复审派发中 |
| [codex]#421-裁定落码-Q1Q2Q3 | kimi编排→codex exec（grilling 三裁 2026-07-28） | 两路喂+闪现窗零寿命+Provisional 可判反超，探针神谕验收（p409 同数据重测基准），工位 /tmp/kimi-nest-mainline | 派发中 |
| [codex]#421-裁定落码-Q1Q2Q3 | kimi编排→codex exec（grilling 三裁 2026-07-28） | 两路喂+闪现窗零寿命+Provisional 可判反超，探针神谕验收（p409 同数据重测基准），工位 /tmp/kimi-nest-mainline | 已交付（26771ee0d8；kimi 抽验：1994/1 唯一红 #491、10 对 cmp=0、IdentityVanished=0；实测闪现率 100% 触发 Q2 逃生门，编排者裁定 (a) 开逃生门，已记票 comment-5103071677） |
| [codex]#421-逃生门-逐bar喂数 | kimi编排→codex exec（逃生门裁定 2026-07-28，comment-5103071677） | nest_lifecycle sidecar 换独立逐 bar 喂数循环（#402 选项 c），Q1/Q2/Q3 语义不变，工位 /tmp/kimi-nest-mainline | 已交付（391936a486；kimi 抽验：1997/1 唯一红 #491、10 对 cmp=0；实测 per-bar 仍 248/248 闪现，p409 反事实 vs 生产结算矛盾，待编排者裁定 comment-5103436732） |
| [codex]#523-结构完成根因调查 | kimi编排→codex exec（编排者 2026-07-28 选 C） | #421 阻塞：248 结构窗首见 bar 即完成根因调查（数据/定义/bug），只读，工位 /tmp/kimi-nest-mainline | 已交付（根因=bug：PanLive provider 接缝缺口，live 迟到非完成过早；报告+248 行 TSV 入库；建议新选项 E 待编排者裁） |
| [kimi]#421-E裁定落地 | kimi编排（编排者 2026-07-28 裁 E） | #421 收窄 destination 记票 comment-5106372198；新开 PanLive provider 票 #527（map #59 子票，L1 先行）；p409 反超口径单列反事实 | 已落地 |
| [codex]#429-三审 | kimi编排→codex exec（E 裁定口径为尺） | 裁定链收口核验 a12a1022d9..391936a486，落 shadow-429-trireview-20260728.md | 已交付（FAIL：Spec PASS + Standards FAIL MED×6/LOW×1；其中 2 条核为 #446/#511 范围污染） |
| [codex]#430-三审 | kimi编排→codex exec（E 裁定口径为尺） | 同上，落 shadow-430-trireview-20260728.md | 已交付（PASS WITH CONDITIONS：无 HIGH、永禁零命中；条件=文档收口×2 + Standards 债去向×3） |
| [kimi]#421-收口包裁定 | kimi编排（编排者 2026-07-28 裁 a） | 文档修 3 + 债票 #532（p123 长函数）/#533（字节门仓内化）+ 污染转 #446/#511 属主，记票 comment-5106781616 | 已落地 |
| [codex]#421-收口包文档修订 | kimi编排→codex exec（收口包裁定 a） | T8/provider 文档口径统一 + rebuild 报告订正注记 + 自查档 E 执行节（L0-L3）+ F6 改名符实 | 派发中 |
| [kimi]#421-关票链 | kimi编排（收口包抽验 58672e2563 全绿） | #429/#430 双关（条件满足+债有去向）→ #421/#427 双关（E 口径五项全绿）；map #59 记 Decisions；三审报告入库 | 已收口 |
