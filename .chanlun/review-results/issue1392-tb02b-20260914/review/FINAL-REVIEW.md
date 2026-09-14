# #1392 TB-02-B 最终产品组合审查

名分：#1392 工作草稿。审查者：`/root/tb02b_incremental_review`。日期：2026-09-14。

结论：**approve_bounded**。未发现本次最终组合新增的正确性、性能或复现工具阻断问题；这不是合入 main、关票或全图完成批准。

## 固定版本与覆盖

- 本次审查固定 HEAD：`d8810bff970118304fde912c0d6e9545a479b6af`。
- 整体比较基线：`6f99b36c64e0560febdb08e5d1f8e781d2af9299`。
- 已有功能审查基线：`5eeaba4ae19ee6cfe038ee4e25eb61fa41779a0f`。
- 整体差异 56 个路径，其中 28 个产品/测试/夹具路径、28 个 `.chanlun` 材料路径。相对已评功能基线的差异为 38 个路径；产品面只有四份已评增量源码、两份 Rust 格式化文件、一个 harness 路径参数调整和三份新增复现工具/冻结 Oracle，没有其他漏入的产品变更。
- 本报告是固定产品树之后新增的评审文档，不属于上述 HEAD，也不计入下方产品哈希，避免自引用。

## 概念层质询结果

通过，结论限定在现有合同与受测域内。复用此前对 `.chanlun/definitions/bi.md` 新笔双坐标、`baohan.md` 组锚及本票已签包含事实合同的核对。全量和增量实际共享 `detect_fractal_at`、`endpoint_at` 与 `ScanState::apply`；包含 facts 外层分支仍分别实现，不把逐句相同称为整体共用一个实现。

四份增量源码已与此前审查 worktree 及 `incremental/VERIFICATION.json` 的 source_sha256 逐字核对一致。检查覆盖初始方向未定不借未来回折、真实极值根与稀疏 raw 序位、右组封口后冻结 checkpoint、EQ 撤笔与 blocked 恢复、确认字段冻结、真实稳定笔前缀及段缓存深回退。正常追加只扫描最多两个 mid；这不保证所有数据结构操作均为常数成本。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

整体业务基线依据为 `review/REVIEW-RAW.txt`、`REVIEW-NORMALIZED.json`、`WORKER-RESULT.json`。封装器原状态仍为 failed；normalized 只恢复格式，没有将其伪装成成功工蜂。采用其可核验代码审查内容，并保留 root_qualifications 的两项撤回：不采用“第三条件为假不可达”和“HEAD 落后 main”。

最终差异复核：

1. `rust/src/bin/s_session_v2/tb02b.rs`、`rust/src/bin/s_structure_session.rs` 均与对已评基线原文件执行 `rustfmt --emit stdout --edition 2021` 的输出逐字一致，未加入行为修改。
2. 四份 parser 源码沿用本审查者此前 approve_bounded；稳定笔前缀与段重建闸分别见 `stroke.rs:600`、`mod.rs:236`，checkpoint 恢复见 `stroke.rs:476`。
3. 复用 `/root/tb02b_python_driver_review` 对故障入口路径参数化的独立审查，并核对 `REPRO-PREPARE-CHECK.json` 中两个文件哈希与最终字节一致。真正复现入口 `run_tb02b_faults.py`、`compare_tb02b_independent.cjs` 未含 `/Users/`、`/opt/homebrew`、`/tmp/` 或 `.scratch` 本机路径；路径来自 CLI/运行计划。历史 `tools/run-faults-r2-original.py` 是明确标识的原件，不能视为可移植入口。
4. 比较器与已运行原始文件逐字一致；冻结独立 Oracle 哈希为 `0450031d1201f64e1cfcd5f347a2597070e6c61f31f9376434382ac09245ac56`。比较器直接核算公开候选条件，不导入产品 reducer。
5. `ARCHIVE.json` 的 13 份绑定全部匹配字节数和哈希，且与原路径原件逐字相同；`CURRENT-ARTIFACTS.json` 的 9 份绑定全部匹配。git diff --check 通过，唯一明确排除项为保持原始字节的 `incremental/PARSER-REGRESSION.log`。

## 当前验证证据与产品字节绑定

本次未重新编译、运行 cargo 或启动 S/Q/Chromium；以下是对已完成验证实体的复核，不冒称本次重跑。

- `current/BUILD-SOURCE.json` 的 439 个源文件哈希已逐一对当前文件核验，失配 0。最终 S 二进制实文件 SHA256：`bd50f6521264d4fdf3cf15ca73a14530e45bc1e5d6045795b1a3882acc25d549`，与构建清单及故障结果一致。构建清单如实标识为 working_tree_build，不能把构建时旧 base_head 冒充最终 HEAD。
- `current/NORMAL-CAPTURE-RESULT.json`：34/34 运行通过，无 failed/not_run；独立比较结果另见 `current/INDEPENDENT-COMPARISON.json`，17 条轨迹、82 个声明前缀、164 arm-prefix、476 公开候选，errors=[]。当前比较器结果绑定的 RUN-PLAN 哈希匹配，34 份 config 均与计划哈希相符且引用上述最终二进制路径。
- `current/FAULT-RESULT.json`：八次运行 exit=0，四个事务点两 arm 的完整语义核心哈希分别相等，difference 均为 null。故障结果绑定上述最终二进制哈希。
- `incremental/VERIFICATION.json`、`SCALING.json`、`FULL-COMPARISON.json`、`PARSER-REGRESSION.log`：108 parser 测试通过、0 失败、8 ignored；固定输入 512/1024/2048/4096 根的 Classification/Tower 四检查点均与全量相同。4096 根追加计时 0.014795042 秒是同输入、同配置的阶段证据，不是整个市场延迟保证，也不是整个 S 会话耗时。
- 原 F1 的逐 bar 全量重建已由真实增量修复和上述量测处置；F2 的比较器、故障驱动与报告原件已入交付树。F3 的主线合入批准、CI、扫描和工位处置仍由后续流程处理，不算新增算法缺陷。

## 有效域与未验边界

业务成功证据限于冻结 Oracle 与报告声明的有限无同价竞争域。增量代码审查另覆盖同价等待/撤回的局部状态边界，但不据此宣布 #1405 同价一般域结清。增量等价域为合法 OHLC、source_index 唯一严格递增、固定配置的逐根追加；历史修订须走已声明的事实代际重建，裸非递增/重复 source 输入不在已证域。

巨大同价根集合的 Vec 复制、保留旧 Rc 快照导致的复制及段扫描成本，不由“最多两个 mid”推出常数复杂度。未证明全部市场输入、S-4 跨实现等价、#1392 全部关票条件或 #1323 总图闭合。旧报告的 not_covered/remaining 保持历史原文；当前报告只依据最终对应实体说明已补的部分。

## 最终产品、测试和夹具 SHA256

以下 28 个路径是整体差异中的全部非 `.chanlun` 文件，哈希直接从本次固定 HEAD 对应的工作树字节计算；其余源文件绑定见 `current/BUILD-SOURCE.json`。

| 路径 | SHA256 |
|---|---|
| `rust/src/bin/s_session_v2/mod.rs` | `719a967993ae1b7f829cede2df8b8cf2cc646acdd3ed7e3d7b7a8dbde53f5e2d` |
| `rust/src/bin/s_session_v2/tb02a.rs` | `e39eca5eafef90fadbff634783899ab96d934dc3137760810ae108439c96c9b3` |
| `rust/src/bin/s_session_v2/tb02a_facts.rs` | `93b24c11550e2ac5ee80197b0ee94816f7e8c63dab6cf9ac5bbe5be9c538a6c4` |
| `rust/src/bin/s_session_v2/tb02b.rs` | `1219e530619f688af85323c582b2c06f29b143a8b5926f6fbe76d82ff2b5cb99` |
| `rust/src/bin/s_structure_session.rs` | `12db2d075192edb76f4b8f701dd4aff956a1104c9fa963ee816382ca74b7c909` |
| `rust/src/theta_v0/parser/fractal.rs` | `3d4c7c40d74225366d7488522d795dafbf24a1149bdf8903de5dbfd11970401c` |
| `rust/src/theta_v0/parser/inclusion.rs` | `77257554c750bae866fab1c74845a1696644f5a61e6c949911ee0853bc1a506d` |
| `rust/src/theta_v0/parser/mod.rs` | `0e24cce81af484cf885c49d75c8fc1876283b5fb00ffb0f5b4549e1b8f89012f` |
| `rust/src/theta_v0/parser/profile.rs` | `1ccc9654bafa70408352adf0da2f9d639d934d93d99553f489e314ee7e5725a7` |
| `rust/src/theta_v0/parser/stroke.rs` | `f96a72fa786bc720e3479165b2972f51452633f640531548aa9d8978d434eee7` |
| `s_session/browser/index.html` | `dcd5e5e927f3ebf379baf56735e9a4fedbefb63616d04846cb091cd4bc5e5de0` |
| `s_session/browser/tb01c-client.js` | `25850fbc6c04c222a387e1de6a860777384d283f14a41672058967675cbc8b98` |
| `s_session/s_query_integrity.py` | `ee1858ef2ff8ee84f5040b6874bc80afb6a6adbdb42b0e394ecf5110aa0d102e` |
| `s_session/s_tb02_contract.py` | `95a8dc1956b0c8153707ccef2c75df78fcf0b8765f8a38ef241d76bf6bcfca93` |
| `s_session/tests/compare_tb02b_independent.cjs` | `6c0eb72779dcd51477d391eb6644136e3237eb40d294fbf18626789fd6c1e0cb` |
| `s_session/tests/fixtures/tb02b/ORACLE.md` | `0ce6b5a96b6e76b7c37a809887b29b7f35bbf9b466ca481549aab0b86a32f168` |
| `s_session/tests/fixtures/tb02b/hand-oracle.json` | `b351af2d9e3faceafe9a1ee69662962887c4ca70d367a2001b3efb2ecb2997bd` |
| `s_session/tests/fixtures/tb02b/independent-source-oracle.json` | `0450031d1201f64e1cfcd5f347a2597070e6c61f31f9376434382ac09245ac56` |
| `s_session/tests/fixtures/tb02b/raw-ledger.json` | `0caafdb71b5df667be3c58781aa28011e5ec949bb128686623929c63078c5169` |
| `s_session/tests/run_tb02b_faults.py` | `339705de95b9481de514b1827e77ed9d29de57b50849aa3aa7f815cd88a17a21` |
| `s_session/tests/tb02a_runtime.py` | `99ea4d4320a612b66756ace3f079338b2c99f650b0900036f6cd910ae0c408ff` |
| `s_session/tests/tb02b_browser.cjs` | `aae278f679051e29fa7206e92f61fb42215f16c36c1d482e273579d8374861f4` |
| `s_session/tests/tb02b_harness.py` | `e7bec8af5fc0d1d9e1fcefe0a0f31de8dd6f442236b5c8deaf19ff33ccff5985` |
| `s_session/tests/tb02b_independent_harness.py` | `17d6c9909863c55539f042bd77f02cc5781bf5d988ac8d2b2e270a0165c70163` |
| `s_session/tests/tb02b_runtime.py` | `36eae96f43d4732e91c95b33c03251674963bf75e3bfdf10378dd50f0e6197b7` |
| `s_session/tests/tb02b_verify.py` | `4dd023735c973ed39cadbbfc1c24ee9b5836bc6cd19d22b6b0656b1d3dadac6c` |
| `s_session/tests/test_tb01c_browser.cjs` | `362974942c28abc11fe65a8a8615590389e9ae7514b1b1115b8c4a600a27c506` |
| `s_session/tests/test_tb02b_contract.py` | `971c0218cb6ec7c0076e67277c55df790ac0b846fa96f2fd350b4af1f96e627f` |

最终结论：**PASS / approve_bounded**，仅适用于上述固定产品字节与声明域。
