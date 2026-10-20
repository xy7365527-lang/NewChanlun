## 复审结论：**REQUEST_CHANGES**（轻度）

测试：`node --import tsx --test .sandcastle/decision-claimer.test.ts` → **59 tests / 59 pass / 0 fail**。

第一轮 4 条 MEDIUM 与 7 条 LOW 的修复本身都成立；LOW-3 保留的理由可接受。但 MEDIUM-3 的修法（T 有事实缺口转 P）引入两个新问题，一个是票上对用户的错误陈述（影子评论首行把「事实缺口转 P」说成「审查仍 BLOCK 后转 P」），一个是升档重跑考古后不再过事实缺口门——与刚堵上的洞是同一个洞的另一条路。两处都是几行改动，修完即可 APPROVE。

## 逐条核对

| 条目 | 结论 | 证据 |
|---|---|---|
| MEDIUM-1 C 类 decision 推荐值 | **PASS** | `decision-claimer.mts:751` 新增校验「C 类 decision 缺推荐答案（推不出就改 gap=probe）」；`:803`、`:809` prompt 改为「B 与 C decision 必填」；`:822` 删掉 `\|\| "见草稿「候选」"` 兜底；`graph-engineering.md:40` 同步。测试 `:802,:804` 以 Q6 断言报错，`:888` 正例改为真实推荐值。三处口径一致。 |
| MEDIUM-2 畸形 JSON 抛错 | **PASS** | `:697-708` `objs()` 过滤非对象/数组元素，`points/queries/hits` 三处统一；`typeof j !== "object"` 也补了。测试 `:791-793` 覆盖 `points:[null,3,…]`、`queries:[null]`、`hits:[null,…]`。 |
| MEDIUM-3 research/probe 缺口无人执行、不阻 T | **PASS（带下文 NEW-MEDIUM-1/2）** | 选了建议 (a)：`:1048 factGaps`；`:1150-1159` T 类有缺口→`initialState("P")`、`convertedToP=true`、`effCls.reason` 附 gapNote、`reclassNotes` 记录；`graph-engineering.md:42` 写明「决定等事实…T 类转 P…缺口票由 /graph-approve 开」；`SKILL.md:13` 补了按 `/wayfinder` 开 `wayfinder:research`/探针票并接 blocking 边。测试 `:918-930`。 |
| MEDIUM-4 升档重跑考古盲跑/不升档/覆盖时长 | **PASS** | `:1016-1027` 新参 `{feedback, model, round}`；`:1163` redo 时传上轮 `ar.problems`、`s.drafterModel`、`s.attempt`；时长键 `archaeology${round}_${i}`（首轮 `1_1/1_2`，redo `2_1/2_2`，不覆盖）；`archaeology.json` 记实际 `model`。测试 `:858-859` 断言第三次调用带反馈且为 Opus。 |
| LOW-1 `gh search issues --state open` 漏网 | **PASS** | `:721 TRACKER_ONE_STATE`，`:738` 组合判断；`--state=open`/`--state open` 均覆盖；测试 Q6。 |
| LOW-2 三处重复 | **PASS** | `:659-662` 代码注释缩为指针到 `graph-engineering.md「2. 考古」`；prompt 保留（合理）。 |
| LOW-3 reviewPrompt 不可达分支 | **未改，接受** | 分支无害；作者理由成立。 |
| LOW-4 无 C 类时 R8 指令悬空 | **PASS** | `:905`「对任一 B/C 点换一个说法再搜一次，搜出更早或相冲突的在案先例即 BLOCK」；`graph-engineering.md:69` 同步。 |
| LOW-5 考古对照/引用自证入正本 | **PASS** | `landPrompt :924`「写七段裁定内容，『引用自证』『考古对照』两段是流程产物，不写进正本」。只是提示约束，无机械裁剪；可接受。 |
| LOW-6 SKILL 缺补考古动作 | **PASS** | `SKILL.md:13` 给出 `--archaeology-only --ticket <号> --out /Volumes/AgentStorage/decisions` 命令。 |
| LOW-7 archaeologyOnly 不取锁 | **PASS** | `:1067-1075` 与 `processTicket :1082` 同一把 `<dir>/.lock`，`wx` 创建、`finally` 释放，`loadTicket` 抛错也会释放。测试 `:937-942`。 |
| LOW-8 续做旧票静默无考古 | **PASS** | `shadowComment :1306-1308` 无考古时写「本票无考古（旧流程产物）：批复前先跑 …」，`--out` 用 `dirname(dir)` 正确。测试 `:932-935`。 |

## 新发现

### MEDIUM

**NEW-MEDIUM-1 事实缺口转 P 的票，影子评论首行对用户说错原因**
- `decision-claimer.mts:1305`：`s.convertedToP ? "（原判 T，${s.attempts} 轮审查仍 BLOCK 后转 P）"`。MEDIUM-3 的修法在 `:1155` 起草前就置 `convertedToP=true`，此类票若审查一次 PASS，票上评论首行会写「原判 T，1 轮审查仍 BLOCK 后转 P」——既没 BLOCK 也不是审查导致。gapNote 只进了 `summary.reclassNotes`（`:1159`）和 `effCls.reason`（给模型看），票上没有任何一行说明「因事实缺口 Q4 转 P」；用户只能从下方「事实缺口（不问你，开票补）」列表反推。新测试 `:918-930` 只断言 `convertedToP === true`，没断言评论文案。
- 建议：给 `Summary`/`RunState` 加区分（如 `convertReason: "fact_gap" | "review_block" | "no_landing"`），或 `shadowComment` 从 `s.reclassNotes` 取首条写首行；同时把 gapNote 作为一行评论写到票上（与 `:1145` 的分级评论并列），否则票面「分级：T」与最终 `needs-approval` 之间无解释。

**NEW-MEDIUM-2 升档重跑考古后不再过事实缺口门，T 类仍可自动落地**
- `:1150-1152` `factGaps` 只在首轮考古后算一次；`:1162-1164` redo 重跑的 `ar` 直接进入第二稿与审查，`nextAfterReview` 对 T 类 PASS 即 `land_t`。触发路径真实存在：首轮某 C 点缺 `gap`（机械校验「C 类须写 gap」）→ 草稿 BLOCK → 升档重跑考古把该点补成 `gap=probe`/`research` → 第二稿 PASS → T 落地，恰是 MEDIUM-3 要挡的「事实缺着、裁定先落正本」。
- 建议：把 gap 判定收成一个函数，在 `:1164` 重跑后再调一次；有新缺口即 `s = { ...s, klass: "P", convertedToP: true }`、`effCls` 改 P、`reclassNotes` 追加（与 `:1187-1189` 改判分支同形）。补一个测试：首轮无 gap 有问题、重跑后出现 `gap=probe`，断言 `phase !== "land_t"`。

### LOW

**NEW-LOW-1 `--archaeology-only` 撞锁时报错形态与 `processTicket` 不一致**
- `:1070` 直接 `throw`，`:1085-1087` 返回 `phase:"error"` 的 summary。CLI 层会把前者当未捕获异常退出。功能上无害，只是两条路对同一事件的报告形态不同；若 `--archaeology-only` 走 `main` 的 try/catch 则可忽略。

**NEW-LOW-2 `landPrompt` 的「不写进正本」仅靠提示，无机械核对**
- `:924` 加了约束，但落地后没有检查正本文件里是否含 `CITATION_SECTION`/`ARCH_SECTION` 标题。LOW-5 的修复按作者选项「landPrompt 明示」成立，记录在此供后续：`landT` 落盘前 `grep` 一次标题即可。

**NEW-LOW-3 origin/main 已前进，合入前需 rebase**（沿用上轮备注，未变）