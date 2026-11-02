# #1778 独立审查（28I，第二轮）：domain-event 持久撤回子契约版本门控

- 审查者：/root/review1778_contract28i。没有参与起草、28G 审查或 28H 返修。只读：没有改代码、GitHub 票、PR、main 或原始材料。
- 审查对象：`/Volumes/AgentStorage/decisions/1778/archaeology.md`、`archaeology.json`、`draft.md`（mtime 2026-10-28T07:39–07:41+08），返修说明 `1778-CONTRACT-REPAIR-28H.md`。
- 基线：`git fetch` 后实测 `origin/main` = `c0c5a369a49852e0fa5a724e57d9adbfab205975`；返修候选 `e1bd49a5b67aea729aa39ad983d93a58db16f72e`。代码一律用 `git show <sha>:<path>` 导出后复核，没有读本地 checkout。
- 已读规程：pstack-upstream-poteto-mode `SKILL.md`、`playbooks/orchestrate.md`、仓库 `AGENTS.md`、`.sandcastle/docs/graph-engineering.md`、`docs/agents/wayfinder-workflow.md`。

## 结论：PASS

28G 的唯一阻塞项 F1 已经闭合。裁定 3 现在只剩一种实现读法：同一会话 manifest 的选择子覆盖 E 和全部 Bᵢ，不设逐域列表，各域在 init 写 meta，在 audit/recover 核对。这和 `Config::load` 的真实结构一致：E 和 B 读同一个文件，只按 `chong` 区分域，`manifest_hash` 相同（main `store.rs:108`、`:276-297`、`:293`）。它也和 `book_revision` 判例同式（`:802-803` init、`:1002-1003` audit，都不按域过滤）。F2 到 F7 也都闭合。下面三条 LOW 是落地时顺手改的措辞和验收细节，不影响裁定内容，不构成阻塞。

## F1–F7 闭合核对

| 项 | 状态 | 证据 |
|---|---|---|
| F1 选择粒度 | 闭合 | 裁定 3 删去了「各自按本域 manifest、互不继承」，改为全会话；考古 Q3（md 与 json）默认值同步，出处换成 `book_revision`。我核对了 main `store.rs:120`（`sr_expansion` 只看 manifest）、`:200-236`（op 校验遍历全部来源，在 `:276` 选域之前完成）、`:293`。所以验收 1 声称「E 与每个 Bᵢ 的 `Config::load` 都报 `InvalidDomain`」在代码路径上成立，不依赖 db 路径。`sr_expansion::bound_revision` 只认 E（`sr_expansion.rs:333-336`），草稿已明写那是该合同自定的范围，不是本合同的模板。 |
| F2 #1776 笔误 | 闭合（见 L1） | 草稿第二节、第六节、边汇总，考古 T15、Q9，以及 json `edges.forbidden_add = ["#1776 blocks #1768 (ticket body item 4 typo)"]` 三处一致。实测 `#1768 blocked_by` = #1534(closed)、#1778(open)；#1776 的 `blocking` 为空；#1776 标题是「[task] #1742 Docker 方案 B 第 13 行受控验收专用票：工蜂写入探针文档」，OPEN。 |
| F3 验收二值 | 闭合 | 原第 1 项的人工核已换成测试 `domain_event_withdrawal_op_unknown_without_selection`，断言错误前缀 `InvalidDomain：操作权限未知或重复`，这个字面量在 main `store.rs:235` 逐字存在。第 2 项首个测试改名后不再和第 1 项重复。其余各项都是命令、测试名、文件存在或票状态。 |
| F4 ADR 0051 不回写 | 闭合 | 验收 7 加了 `git diff --quiet origin/main -- docs/adr/0051-…md`；子合同落新 ADR，号在落地时取。第七节写明 N-1 不授权回写 ADR 0051。ADR 0051:8 原文「本条依用户批复落地，不以 #1323 Notes N-1 代替」逐字成立；该文件在 main 上的最后提交是 `7b842b69a1`。 |
| F5 旧二进制 | 闭合 | 新增裁定 10。我核对了 main：`:108-125` 不对未知顶层键做 exact 校验，`:969-1015` 只核已登记的 meta 键，所以旧二进制会忽略新字段。来源授予 `WithdrawDomainEvent` 时，`:207-236` 的封闭集合不含它，在 `:235` 加载即拒。授权集合随 `manifest_hash` 冻结（`:975` 核 meta），所以带撤回条目的库换成不授予该 op 的 manifest 会先报 `IdentityConflict`。「§5.8 的拒绝靠未知 op 授权间接实现」这个表述准确。 |
| F6 行号与判例 | 闭合 | main `store.rs:1132` 是 else 分支，`:1141` 是 E 判据，`:1147` 是 B 判据，`:946` 是「`/1` 恰好 10 表……§5.8 旧域不混写」，`finance.rs:637` 是「§5.8 旧域不混写」，`finance.rs:20-22` 是 `REV_V3`（`/1` 域上显式选择的窄测试臂），`:55` 是 `revision: REV_V3`。e1bd49a `store.rs:230` 无条件列入 `WITHDRAW_OP`，`:1100-1103`、`:1228`、`:1369-1371`、`:1426-1428` 四处分支起点都对；`service.rs:1462` 无门控分派。CONTEXT.md `:460` 引句逐字成立。 |
| F7 map 索引与 N-1 | 闭合 | `gh api …/issues/1778/parent` 返回 404。#1323 body 共 70,804 字节，`:88` 是 `## Decisions-so-far`，`:92` 是 #1534 那一行。graph-engineering.md `:77`（T 类落地手续）和 `:90`（N-1 原话）都和第七、八节的引用一致。第八节没有伪造原生 sub-issue，只给索引行建议文字，由主控在合入后补。 |

## R1–R8

| 项 | 结论 | 依据 |
|---|---|---|
| R1 引用回查 | PASS | 抽查 §5.8、ADR 0051 `:7-8`/`:236`/`:374`、ADR 0041、store.rs、sr_expansion.rs、finance.rs、book.rs、e1bd49a 四文件、#1768 body `:19`/`:45`，全部成立。#1778 body 第 4 项已在审查材料落盘后被更正（L1），是事实漂移，不是引错。 |
| R2 不与正本冲突 | PASS | 并列子合同符合 §5.8「不重写、不重铸」和 `store.rs:17-18`。全会话选择和 manifest 结构一致。#799 和 #804 都没有被违反：门控是单一判据，没有宽严两档；写路径和审计共用 `withdrawal_image`。ADR 0041 已判为不适用，理由成立（`0041:9` 只管缠论自由度）。 |
| R3 验收二值 | PASS | 八项都是命令、测试名、文件存在或票状态。细节建议见 L3。 |
| R4 不越 Destination、不夹带 P | PASS | 不涉及资金、场所、权限或启用。N-1 只用于新决策和新 ADR，ADR 0051 有 diff 门锁住。CONTEXT.md 只追加一句指针，原句不动。 |
| R5 无宽严两档 | PASS | 已选择才认识该 op，没有回落路径。未绑定域出现撤回条目直接判 `StorageUnavailable`。这个前置比 sr_expansion 的四处分支更显式，但 sr_expansion 在 replay 里也以 `config.domain != "E"` 报 `StorageUnavailable`（`sr_expansion.rs:634` 附近），判例方向一致。 |
| R6 票与边 | PASS | 实测 #1768 `blocked_by` = #1534(closed)、#1778(open)，`blocking` = #1361、#1363、#1399，父票 #1366。草稿写增 0、删 0、禁增 #1776→#1768，和实测一致。 |
| R7 分级 | PASS | 处理的是事件 schema、seam 和恢复语义，属于 graph-engineering.md:23 列举的 T 类。N-1 的适用范围靠 #1778 body 首行「Part of #1323」，草稿已照实写明。 |
| R8 考古 | PASS | 复跑四条：R2（main 的 `docs/adr`、`.chanlun/definitions`、`CONTEXT.md` 中「子合同或子契约」命中 0）、R4（main 中 `WithdrawDomainEvent` 等命中 0）、R3（`book.rs:7`、`finance.rs:18/:23/:40`、`sr_expansion.rs:9` 等命中成立）、T13/T16（依赖与 parent 404）。换说法复搜 Q3 和 Q1：tracker 搜「持久撤回」只命中 #1778、#1768 和 S 侧的 #1380，搜「manifest 选择 子合同 全会话」没有既有裁定；仓内 `withdrawal_revision` 命中 0，「全会话」只有 SPEC 输入里的无关用法。没有遗漏的已裁定项，C 类为 0 的判断成立，「考古对照」覆盖 Q1–Q12。 |

## LOW（不阻塞，落地时改）

**L1 #1776 笔误已被主控更正，草稿和考古仍用现在时。** GraphQL `userContentEdits` 显示，#1778 body 在 2026-10-27T23:17:14Z 和 23:17:32Z 两版的第 4 项是「建立 #1776 blocks #1768 的依赖」，在 23:42:02Z（即 07:42+08，晚于 28H 落盘的 07:40:59+08）被改成「确认既有 #1778 blocks #1768 依赖（禁止新增 #1776→#1768 边）」。草稿第二节、第六节和考古 T15、Q9 写的是「票面第 4 项写成……」「票面由主控更正」。发 resolution 前请改成过去时，并注明更正时间。`forbidden_add` 不变，和更正后的票面一致。

**L2 裁定 5、6 的「本域已绑定」可以写得更准。** 全会话语义下，加载期已知集合只能看 manifest 选择（main `:232-233` 同式），meta 是 audit 期的事实。因为 audit 先强制「meta 等于 manifest 选择」，两种读法行为相同，所以不是多义。建议写成「本会话已选择，且本域 meta 已在 init 绑定」，让实施者不必推这一步。

**L3 验收细节。** 第 1 项的 `grep -c … *.rs` 在多文件时输出逐文件计数，「合计 ≥ 1」需要手加，建议改成 `cat rust/src/economic_session/*.rs | grep -c …` 或 `git grep -l`。第 5 项「现有 Python 测试通过」沿用 #1768 body `:19` 的写法，没有给命令，建议点名 pytest 路径。第 7 项「不改原句」没有机械判据，建议加 `git diff --numstat origin/main -- CONTEXT.md` 的删除列为 0。

## 代码审查报告

### 概念层质询结果
通过。没有缠论教义概念介入，本票是工程版本门。§5.8、#799、#804、ADR 0041 适用范围都核对过，没有矛盾。

### 工程层审查
| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 3 | note |

结论：PASS。
