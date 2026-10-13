# #1342 受影响代码清单（初始方向与全包含前缀）

> 票：[#1342](https://github.com/xy7365527-lang/NewChanlun/issues/1342)；教义正本：[`baohan.md`](../../definitions/baohan.md)（包含；落点 「合并方向」节初始方向、「未定前缀的输出与身份」节、「#1342 受影响代码清单」节）。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**。行号于该基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码（本次提交仅有的一处代码改动是 #1344 的一行零行为注释，见 #1344 清单）。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/theta_v0/parser/inclusion.rs:41/:52/:80` | main `465174240b` | 不改：`contains`／`merge`／`strict_dir` 为已裁原语 | — |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:110/:129/:430` | main `465174240b` | 旧 batch／`IncrInclusion` raw 扫描与激活重折：收编为本规则的实现基础或写明旧用途；不得在失败时切回 | TB-02-U #1393 |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:555/:563/:588/:603/:627/:761` | main `465174240b` | 方向证据、组成员／极值根、未定门：补 pending → direction-established 的完整分支 | #1393 |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:806/:821/:843/:920/:926` | main `465174240b` | 紧凑状态的永久 unsettled（`:843` 遇包含即停）→ 非包含对出现即解锁并自起点重折；恢复材料同源 | #1393 |
| Rust | `rust/src/theta_v0/parser/inclusion.rs:163/:216` | main `465174240b` | 原始序号 ↔ 组锚、稠密标注：未定材料不伪造正式归组 | #1393 |
| Rust | `rust/src/theta_v0/parser/mod.rs:152/:168/:197/:328` | main `465174240b` | 正式装配与全量 facts 观察同源 | #1393 |
| Rust | `rust/src/theta_v0/parser/stroke.rs:180/:356/:608` | main `465174240b` | 初始化未定时不偷产端点或新笔成功 | #1393 |
| Rust | `rust/src/bin/s_session_v2/tb02a_facts.rs:113/:334` | main `465174240b` | 同源观察：未定对象、waiting 原因、`replaces`、AsKnown | #1393 |
| Lean | `formal/Origin/SegmentFeatureSeq.lean:143/:153` | main `465174240b` | 不改：特征序列的 processUp，不是 raw 初始化 | — |
| Lean | `formal/Origin/SegmentAutoConstruct.lean:69` | main `465174240b` | 同上 | — |
| Lean | `formal/Origin/（新文件）` | main `465174240b` | 补 raw 初始化规则的具名定义与分支证明（空／单根／首对／全包含） | #1393 或 Lean 跟进票 |
| Python | `src/newchan/a_inclusion.py:23/:54/:90/:125/:138` | main `465174240b` | #1286 冻结对照；记录与本规则的差异，不升为生产重判 | — |
| Doc | `.chanlun/definitions/baohan.md` | main `465174240b` | 三行头、初始方向、「未定前缀的输出与身份」节、内嵌清单、未结算问题 3（本次提交已落） | 本票落文档 |
| Doc | `CONTEXT.md` | main `465174240b` | 词条「未定方向前缀」（本次提交已落） | 本票落文档 |

## 本轮复核订正（相对工位草稿）

- 根裁定把「用随后首个非包含严格对的方向折叠此前全包含前缀」定为 `[新缠论:选择]`，并明写长期未定的输入保持未定、不另造默认方向；正本与 CONTEXT 词条按此措辞。
- 工位草稿拟随 #1393 加的 `inclusion.rs` 带票号注释本次不做，随 #1393 实施票落。

基线说明：工位草稿以 main `f95a7f3e` 为基线（#1344 部分行以 TB02D `03239158` 为基线）；本清单不靠差异推断，全部行号直接在 main `465174240b` 上用 `git show` 逐行核过。
