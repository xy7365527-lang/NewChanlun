## 结论：三条必改全部改到位，**overall PASS**（R1–R8 全过）；剩一处可补的字面空白（序列节点）和三条小建议，都不构成阻塞。

### (1) review2 must_fix 1–3 复核

| # | 状态 | 我亲自复跑的证据（基线 `6c6acd5fe0`） |
|---|---|---|
| 1 认证单位改为 (摘要, Flavor, Mode) | **done** | `s_content.rs:259` 是 `pub enum Mode { Public, Template }`，文档注释写明「模板占位只在显式模板上下文里允许」；`:1977-1980` 是 `AuditMemo` 的 `seen: WorkSet<(NodeDigest, Flavor)>`，注释「已核过的 (摘要, 上下文)」，逐字一致。Mode 不在 `seen` 键里，但 `CaptureSource::verify_refview`（`:3145-3155`）按 Mode 把 memo 分成 `public`/`template` 两个槽，等价于三元组单位——第三稿写的单位与现役核验器**同判**。占位拒绝点在 `:5166 if self.mode != Mode::Template`，与草案「Public 模式拒模板占位」一致。验收 1 负例已加「同摘要换上下文」「同摘要换模式」「早先落库未可达节点被新根引用」三条。 |
| 2 根集合补 scope | **done** | `content_manifest.rs:1226-1231`：先 `verify_ref(metadata.scope, Mode::Public)`，再 `generated_state` Public，再 `verify_many_ref([publication_template, delta_template], Mode::Template)`——草案「四个根、各自模式」逐项对应。折叠入口「历史各代的根取自已发布 manifest 记录」成立：scope 是 `ManifestMetadata` 字段（`:103`）且序列化进记录（`:375`）。 |
| 3 scrub 字段落记录与取值域 | **done** | 改名 `audit_ledger_scrub = {interval_generations: K, coverage: c}`，落 S 写者资源档（与 cold_start_window 同记录，`/2` 预留或 P6 升 `/3`）。对照 ADR 0041 第三条：①有具名版本化记录 ②默认「不开」附出处（G4 优先、用户原话）③可选值集合与代价写明 ④「改值即新版本，运行中不换值」。判别句：同一个单位核验函数、只决定何时重读多少 ⟹ 参数而非两档，合规。 |

### (2) 增量重审要点

**止步规则与全闭包核验器的同判性（R2/R5 核心）**。第三稿把止步条件改成「摘要、上下文、模式三者都相同才停；摘要同但上下文或模式不同的按新单位重核」，这与 `run()` 里 `seen.insert((digest, flavor))` 加 Mode 分槽的实际行为完全一致，#1481 第 24 行「异位置同摘要按新上下文重核」也对上了。上一轮的「更松」已消除。

用户特别问的**序列节点「按摘要折叠」**：核验器的 `seen` 只装**值节点**（`push_work` 只推值节点摘要）；字节块序列、元素序列、字段序列不进 `seen`，而是在所属值节点处理时由 `byte_agg`/`elem_count`/`field_agg` 整体折叠，其中字节序列的 memo 键是 `(摘要, Utf8State, NumState)`——带进态，意味着同一字节子树在不同 UTF-8 进态下结论可能不同。第三稿对序列节点**一个字没写**。按它自己的定义（单位 = `seen` 的去重单位）序列节点不是认证单位、不进 node 分区、不止步，随所属值节点整体核——这样 Δ 读法在序列上只会比全闭包**不更松**（共享序列子树可能多走一遍，结论相同）。但表头写的是「节点摘要」，而本仓「节点」一词含序列节点，实施者若把序列节点也塞进 node 分区并按 (摘要, Flavor, Mode) 止步，字节序列共享子树在不同进态下就会比全闭包松。这是**字面空白而非矛盾**，定义锚已把它排除，我列为 suggest 第 1 条（加一句话 + 一个负例），不列 must_fix。

**scrub 字段**：见上表第 3 行，合规。

**关票门 1**：改为固定标记 `startswith("**用户批复（#1644 spec 批准）**")`，二值可执行；所引先例评论 6307519196 确实由 xy7365527-lang 以 `**用户 2026-10-21 批复**` 开头。副作用检查：若「spec 修订草案」评论尚不存在，`T0` 为 null，`created_at > "null"` 对任何日期串都为假，门 **fail-closed**，没有误放行。

**review2 的 6 条 suggest**：1（固定标记）、2（#1491 登记改为「批准后追加评论」）、3（64 GiB 账扩为含台账，写入 #1481 分工段）、4（APPROVED-SOURCE :91 归属一句话——我复跑 `12f9eb1adb` 版 :91 第 4 节第 4 条确有「暖/冷/超预算缓存中的损坏拒绝」）、5（K14 补 #1340 A 锚——只写进 spec 的 K 表，`archaeology.md` 本体 mtime 仍是 13:01 未动）均已处理；6（K13 tracker 触顶注明）archaeology.md:7 原本就有「GitHub 中文分词宽松，30 即触顶」，算已覆盖。

```json
{"R1":{"verdict":"PASS","basis":"新增引用全部按 6c6acd5fe0 亲自复跑成立：s_content.rs:259 pub enum Mode{Public,Template}（注释「模板占位只在显式模板上下文里允许」）、:1977-1980 AuditMemo.seen: WorkSet<(NodeDigest, Flavor)>（注释「已核过的 (摘要, 上下文)」）、:3145-3155 verify_refview 按 Mode 分 public/template 两个 memo 槽、:5166 Public 拒 0x08、content_manifest.rs:1226-1231 先核 scope(Public) 再 generated_state(Public) 再 verify_many_ref 两模板(Template)、scope 在 ManifestMetadata :103 并序列化 :375、APPROVED-SOURCE@12f9eb1adb :91 第 4 条「暖/冷/超预算缓存中的损坏拒绝」、#1612 评论 6307519196 以「**用户 2026-10-21 批复**」开头且 login 为 xy7365527-lang、ADR 0041 第二/三条全文"},
"R2":{"verdict":"PASS","basis":"认证单位 (摘要, Flavor, Mode) 与现役核验器 seen 键 + Mode 分槽等价，止步规则「三者都同才停、摘要同上下文/模式异则重核」与 #1481 第 24 行一致，不再构成 #799 两档；audit_ledger_scrub 按 ADR 0041 第三条四项（具名记录、默认值+出处、可选值集合+代价、改签即新版本）落齐，判别句判为同一核验函数的覆盖参数而非两档；残留：序列节点不在 seen 中、spec 未明写其不入 node 分区（定义锚已排除，列 suggest）"},
"R3":{"verdict":"PASS","basis":"关票门 1 改固定标记 startswith，二值；T0 缺失时 created_at>\"null\" 恒假、门 fail-closed；门 2–5 与 P6 验收 1–8 命令可执行；验收 1 负例新增三条覆盖换上下文/换模式/未可达节点新可达"},
"R4":{"verdict":"PASS","basis":"三个要定覆盖；scrub 字段默认不开、挂问 4，未越 Destination；四根口径与 G4 关系未扩范围"},
"R5":{"verdict":"PASS","basis":"Δ 读法在 Template/Public、Value/JsonInner 边界与全闭包同判；根集合补 scope 后与 content_manifest.rs:1226-1231 实际核的根一致；无 Δ→全闭包兜底、stale 不重试、scrub 发现即 stale 仍成立；序列节点按 spec 定义不止步、Δ 只可能不更松"},
"R6":{"verdict":"PASS","basis":"边关系未变仍无环；#1491 改为「批准后追加登记评论」解决 tracker 不存在的缺口；#1481 分工段新增 64 GiB 账内涵扩展；#1640/#1641 字段表归属已写"},
"R7":{"verdict":"PASS","basis":"P 类，经 spec 批准闸；:218 单列待批未变；本稿不改 #1340/APPROVED-SOURCE 原文"},
"R8":{"verdict":"PASS","basis":"K13/K14 数字与 review2 复跑一致；K14 补 #1340「S 自身存储损坏另按结构故障处理」A 类锚（只在 spec K 表，archaeology.md 本体未同步）；K13 触顶说明 archaeology.md:7 已有"},
"must_fix_status":{"1":"done（单位=现役 seen 键 (NodeDigest, Flavor) + Mode 分槽，:259/:1977-1980/:3145/:5166 复现；验收负例三条已加）","2":"done（四根按 content_manifest.rs:1226-1231 顺序与模式逐项对应；scope 在已发布 manifest 记录 :103/:375）","3":"done（audit_ledger_scrub={K, c} 落 S 写者资源档 /2 或 /3，ADR 0041 第三条四项齐；覆盖参数非两模式）"},
"overall":"PASS",
"must_fix":[],
"suggest":["裁定一/裁定四(b) 加一句：序列节点（字节块、元素序列、字段序列）不是认证单位、不入 node 分区、不止步，随所属值节点整体折叠核验（现役 seen 只装值节点，byte_agg memo 键含 Utf8State/NumState，s_content.rs:1985-1987）；验收 1 负例加「共享字节序列子树在不同 UTF-8 进态下被两个字符串引用」","archaeology.md K14 条本体同步写入 #1340「S 自身存储损坏另按结构故障处理」A 类锚（目前只在 spec 第九节 K 表，考古件 mtime 13:01 未动）","关票门 1 可再加 user.login == 'xy7365527-lang' 过滤，防他人评论以同标记开头（当前仓只有主控一个 login 发评，风险低）","裁定八 audit_ledger_scrub 的 c 建议补「轮转顺序按 node 分区主键字典序」一句，使 p6_ledger_fold_equals_online 在开 scrub 时仍可逐字节对拍"]}
```