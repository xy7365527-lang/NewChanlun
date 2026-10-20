## 第三轮确认：**APPROVE**

测试：`node --import tsx --test .sandcastle/decision-claimer.test.ts` → **61 tests / 61 pass / 0 fail**（第二轮 59 → 新增 2：升档重跑后缺口门、`flowSectionsIn`；另一条既有测试扩了断言）。

HEAD `86f1a3075e` 只动 `decision-claimer.mts`（+37/−8）与测试（+23/−1），范围与提交信息一致。第二轮两条 MEDIUM 均已按建议修到位，新增一条 LOW 级观察（续做路径未带 `factGapNote`），不阻合入。

## 逐条核对

| 条目 | 结论 | 证据 |
|---|---|---|
| NEW-MEDIUM-1 影子评论首行说错转 P 原因 | **PASS** | `Summary.factGapNote`（`:962`）；`gapGate` 内置 `summary.factGapNote = note` 并把 note 作为一行评论写票（`:1167-1168`，dry-run 进 `plannedWrites`）；`shadowComment :1317` 优先取 `factGapNote`，否则才落回「N 轮审查仍 BLOCK 后转 P」。测试 `:930-933` 断言 plannedWrites 含「评论 #1531：考古有事实缺口 Q4」、首行匹配「原判 T，考古有事实缺口 Q4」且不含「审查仍 BLOCK」。票面「分级：T」与 `needs-approval` 之间的解释也补上了。 |
| NEW-MEDIUM-2 升档重跑考古后不过事实缺口门 | **PASS** | 判定收成闭包 `gapGate()`（`:1158-1169`）：`factGaps(ar.arch)`，`s.klass !== "T"` 或无缺口直接返回；否则 `s→P / convertedToP / effCls / reclassNotes / factGapNote / 评论` 一次做完。首轮 `:1170` 调一次，redo 重跑 `runArchaeology` 之后 `:1177` 再调一次。测试 `:936-946` 复现第二轮描述的路径：首轮 Q3 缺 `gap`→机械问题→审查 BLOCK→redo 重跑得 `gap=probe`→断言 `phase !== "land_t"`、`klass === "P"`、`factGapNote` 含 Q3。我核对了 fake 的 arch 序列 `[noGap, noGap, probe]`：首轮两次都带问题（重试耗掉 2 个），redo 第一次干净即 break，序列成立。 |
| NEW-LOW-1 `--archaeology-only` 撞锁抛错形态 | **未改，接受** | `main :1497-1504` 逐票 `try/catch`，失败只 `log` + `exitCode=1`，不会未捕获退出。与第二轮备注一致。 |
| NEW-LOW-2 「不写进正本」无机械核对 | **PASS** | `flowSectionsIn(text)`（`:1327-1329`）按标题正则（`#{1,6}`，容许序号前缀）查 `引用自证`/`考古对照`；`landT :1349-1351` 在越界检查之后、`git add` 之前读 `join(wt, target)`，命中即 `throw "正本混入流程段：…"`。测试 `:948-951` 正反例（`goodDraft` 本身含两段，按设计正确报出）。 |
| NEW-LOW-3 rebase 到最新 origin/main | **PASS** | `git fetch` 后 `origin/main = 6badaa6049`，`merge-base HEAD origin/main = 6badaa6049`，`origin/main..HEAD` 恰 3 个提交，线性。 |

## 新发现

### LOW

**NEW3-LOW-1 续做路径未恢复 `factGapNote`，窄窗口内影子评论仍会写错原因**
- `processTicket :1129-1133` resume 分支 `Object.assign(summary, {...})` 列举了 `klass/convertedToP/reclassNotes/...`，没带 `p.factGapNote`。触发路径：事实缺口转 P 的票在 `shadow_p_pending` 持久化后、`comment` 步 `gh issue comment` 失败（`pDone=[]`，`finally` 落盘）→ 下一轮续做 → `shadowComment` 走 `convertedToP` 分支，首行写「原判 T，1 轮审查仍 BLOCK 后转 P」——正是 NEW-MEDIUM-1 要消除的文案。
- 窗口很窄（只有第一步 comment 失败且后续重跑才会碰到），且 gapGate 已经单独写过一行事实缺口评论到票上，用户有信息可对照，故记 LOW。修法一行：`Object.assign` 里加 `factGapNote: p.factGapNote`；或 `shadowComment` 回退时取 `s.reclassNotes?.[0]`（续做已带 `reclassNotes`）。

**NEW3-LOW-2 `正本混入流程段` 抛错后的续做会重复落地同一份草稿（沿既有模式，记录即可）**
- `landT` 抛错时 `stage=reviewed_pass` 已持久化，下一轮从 `land_t` 续做，再跑一次落地工蜂；若工蜂对同一草稿稳定地把两段抄进去，会每轮失败一次直到人工介入。这与既有的「落地越界改动」「落地工蜂未改动正本」抛错行为同形，不是本次引入的新形态；后续可考虑在 `landT` 失败前先把 `draft` 中两段机械裁掉再交工蜂，或失败 N 次后降为 P。

无 MEDIUM 及以上新问题。两条 MEDIUM 修复的逻辑、测试覆盖与票面文案均核实，建议合入；NEW3-LOW-1 可随下一次改动顺手补。