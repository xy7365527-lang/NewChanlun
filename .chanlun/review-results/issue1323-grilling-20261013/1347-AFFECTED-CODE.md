# #1347 受影响代码清单（Type2/3 NA 在递归证书中的合成）

> 票：[#1347](https://github.com/xy7365527-lang/NewChanlun/issues/1347)；教义正本：[`qujiantao.md`](../../definitions/qujiantao.md)（区间套；落点 §8.2.4、§9 第 53／66／67／68 条、§9.1）。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**。行号于该基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码（本次提交仅有的一处代码改动是 #1344 的一行零行为注释，见 #1344 清单）。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/theta_v0/classifier/nest.rs:158-168` | main `465174240b` | `NestRung`（`cand: bool` 在 `:167`）升三值或加适用性；两个构造器签名随改 | TB-02-X #1442 |
| Rust | `rust/src/theta_v0/classifier/nest.rs:305-336/:339-360` | main `465174240b` | `n_delta` docstring 与 `n_delta_rec`：只服务 Type1；加带票号说明 | #1442 |
| Rust | `rust/src/theta_v0/classifier/nest.rs:428-443/:893-955/:958-1026` | main `465174240b` | 链 B typed 证书（`:997` `assembled(.., true)`）：rung 的 `true` ＝ 已求值 Cand；Type2/3 入口分派到条件证书；`chain_lookup` 携带入口点型 | #1442 |
| Rust | `rust/src/theta_v0/classifier/level_view/confirm.rs:111-147` | main `465174240b` | `NestCandidateEvent` 无入口点型字段：加字段或由 gate 侧携带；`:118` 是「Cand ＝ dir ∧ Comparable ∧ Extreme」的出处 | #1442 |
| Rust | `rust/src/theta_v0/classifier/nest_index.rs:272/:306` | main `465174240b` | 索引按点型消费两类证书 | #1442 |
| Rust | `rust/src/theta_v0/backtest/econ_positive/cand_delta.rs:149-177` | main `465174240b` | `:174` `Type2 \| Type3 => true` 臂在条件证书落地后退场；`:151-152` docstring 补退场条件 | #1442 |
| Rust | `rust/src/theta_v0/backtest/econ_positive/cand_delta.rs:188-216` | main `465174240b` | `cand_delta_base_gate` 即条件证书的基例确认：不改 | — |
| Rust | `rust/src/theta_v0/backtest/econ_positive.rs:381-385/:408-412/:795-874/:1050-1052` | main `465174240b` | 准入按点型取证书；`:411` `nest_depth` → `L_geo`，新增 `D_crit`；`:412` `origin_level = e + D_crit`；`effective_nest_depth` 与 `D_crit` 一致 | #1442 |
| Rust | `rust/src/theta_v0/backtest/admission.rs:261-322/:1103-1104` | main `465174240b` | `nest_gate_admit` 按点型；`:1104` rung 长度量即 `L_geo` | #1442／TB-04 |
| Rust | `rust/src/theta_v0/classifier/turn_class.rs:796/:1211` | main `465174240b` | 测试期望按新规则复核；「sidecar 不进证书真值」不变 | #1442 |
| Rust | `rust/src/theta_v0/strategy/nest.rs:87-93/:126-146` | main `465174240b` | 链 C `candidate_ok`（正本判死字段）与 `chi_bool`：随 #1442 同步或登记退役 | #1442 |
| Lean | `formal/Origin/NestingCertificate.lean:246/:293-298/:307` | main `465174240b` | 引入适用性或入口点型；另立 Type2/3 证书定义；证 Type1 的 `N` 不变 | #1442 |
| Lean | `formal/Origin/IntervalNestCertificate.lean:419-434/:443` | main `465174240b` | 同上（列表链骨架 `nestCertB`／`chiBool`） | #1442 |
| 谱系 | `.chanlun/genealogy/settled/673-cand-delta-predicate-scope-misuse-type1-divergence-vs-type2-completeness-h2-overfiltering.md` | main `465174240b` | 补退场条件（`qujiantao.md` §8.2.4 第 7 条） | #1442 |
| Python | `src/newchan` | main `465174240b` | `n_delta`／`nest_cert`／`NestRung`／`cand_delta` 检索零命中，不涉及 | — |
| Doc | `.chanlun/definitions/qujiantao.md` | main `465174240b` | 三行头、§8.2.1、§8.2.4、§9 第 53／66／67／68 条、§9.1（本次提交已落） | 本票落文档 |
| Doc | `CONTEXT.md` | main `465174240b` | 词条「条件证书」「判据深度（D_crit）与几何链长（L_geo）」（本次提交已落） | 本票落文档 |

## 本轮复核订正（相对工位草稿）

- `qujiantao.md` §9 第 53／66／67 条在 #817 时的行号已漂，正本已就地订正：`NestRung` 现为 `nest.rs:158-168`（`cand` 在 `:167`）；链 B 现为 `nest.rs:997`；链 A 的 `=> true` 随 #1175 拆分迁到 `econ_positive/cand_delta.rs:174`（docstring `:151-152`）；`nest_depth`／`origin_level` 现为 `econ_positive.rs:411`／`:412`。
- 根裁定对第一问的收窄（几何链与 NA 向量只作观察附件）工位草稿已含；实施时不得把 ⊆ 边放进 Type2/3 证书的合取。
- `formal/Origin/CertGatedExit.lean:683`（`xzd : Bool` 形参）的改名义务属 `qujiantao.md` §8.1，不在本票清单。

基线说明：工位草稿以 main `f95a7f3e` 为基线（#1344 部分行以 TB02D `03239158` 为基线）；本清单不靠差异推断，全部行号直接在 main `465174240b` 上用 `git show` 逐行核过。
