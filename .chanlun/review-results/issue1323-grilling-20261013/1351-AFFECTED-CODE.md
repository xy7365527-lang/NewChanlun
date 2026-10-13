# #1351 受影响代码清单（首个具名场所账户与验证权限 profile）

> 票：[#1351](https://github.com/xy7365527-lang/NewChanlun/issues/1351)；裁定正本：[ADR 0028](../../../docs/adr/0028-first-capability-verification-profile.md)。
> 基线：**main `465174240b989a1d99f99a047fbb46b3ee215f49`**（标 main）；#1394 财务线 **FIN `0d8a6017231348e2c4a005f68d4e2e7a457a59fe`**（`codex/1394-transport-revision-20261012`，未合入 main，标 FIN）。行号于各自基线逐行核过。
> 名分：#793 关票判据所需的受影响代码清单，是工作草稿（宪法 §5），活期绑定本票；改码归下表「归属」列的实施票，本清单本身不改任何代码。

| 侧 | 文件:行 | 基线 | 处置 | 归属 |
|---|---|---|---|---|
| Rust | `rust/src/economic_session/finance.rs:204-213` | FIN `0d8a6017` | 现状：资金政策字段表，`:211` 为 `capability_id`（TestOnly）；无 venue/endpoint/credential/environment/account。P1 落地时 `capability_id` 的来源改为 profile 版本 | TB-08（#1382/#1400） |
| Rust | `rust/src/economic_session/finance.rs:225-229` | FIN `0d8a6017` | 现状：`model` 固定 `TestOnlyUnleveragedCashBuy`（`:226`）；真实场所能力进入时需新 model 名与 `UnsupportedCapability` 分支 | TB-08 |
| Rust | `rust/src/economic_session/finance_tests.rs:126` | FIN `0d8a6017` | 现状：TestOnly 夹具 `capability_id=TestOnly-no-leverage-no-venue` | TB-08 |
| Rust | `rust/src/economic_session/` | main `465174240b` | 缺口：`git grep -i -E` 检索 venue、endpoint、credential、environment、account_id 在 main 零命中（FIN 只命中上一行夹具字符串）；profile 类型待 TB-08 新增 | TB-08/09 |
| Python | `trading_system/config/broker_config.py:9-10` | main `465174240b` | 可收编：`HYPERLIQUID_PK` / `HYPERLIQUID_TESTNET_PK` 变量说明；P1 要求 testnet 凭据为 API wallet 而非主钱包，变量语义须随之改 | TB-08/09 |
| Python | `trading_system/config/broker_config.py:59-60` | main `465174240b` | 现状·外效记录：注释记载主钱包已做 builder fee 授权（可撤销），未核实；P1 不依赖 builder code | — |
| Python | `trading_system/config/broker_config.py:74` | main `465174240b` | 可收编：`testnet` 参数选 TESTNET/MAINNET；P1 要求 environment 在 profile 版本内不可切换 | TB-08 |
| Python | `trading_system/live/hl_verify_nautilus.py:128` | main `465174240b` | 可收编不继承：读 `HYPERLIQUID_PRIVATE_KEY`（主钱包私钥口径）；P1 只允许专供 testnet 的 API wallet，主钱包私钥不进运行时 | TB-08/09 |
| Python | `trading_system/live/hl_verify_sdk.py:87` | main `465174240b` | 同上 | TB-08/09 |
| Python | `trading_system/execution/risk_controls.py:12` | main `465174240b` | 可收编：旧 kill switch；急停触发/执行分离后须经正式 command 进入 X（ADR 0030） | TB-08-C #1412 |
| Python | `trading_system/execution/risk_controls.py:58` | main `465174240b` | 不继承：单笔 ≤ 权益 1% 为 #1283 Q3 旧参数 | — |
| JS | `s_session/browser/tb03a-client.js:179-181` | FIN `0d8a6017` | 现状：客户端校验政策字段集（含 `capability_id`）与 `model` 固定值（`:181`）；profile 化后同步 | TB-08 |

## 本轮复核订正（相对工位草稿，基线 f95a7f3e → 465174240b）

- 工位草稿所列 `rust/src/theta_v0/live/exec.rs`、`rust/src/theta_v0/live/feed.rs`、`trading_system/live/hl_paper_runner.py` 在 main `465174240b` 与 FIN `0d8a6017` 上都不存在：它们只见于 #1319 收口记录（`.chanlun/review-results/issue1340-publication-20260909/payload/inputs/LEGACY-TICKET-DISPOSITION.json:2965`），代码未入仓。本清单改列 main 上实际存在的 `hl_verify_nautilus.py:128`、`hl_verify_sdk.py:87`。
- 工位草稿把 `tb03a-client.js:179-181` 标为 main；main 的这三行是请求构造，校验政策字段的是 FIN 同号行，已改标 FIN。

基线迁移说明：工位草稿以 main `f95a7f3e` 为基线；`git diff f95a7f3e 465174240b` 在本清单涉及的全部路径上为空，main 行号因此不漂。
