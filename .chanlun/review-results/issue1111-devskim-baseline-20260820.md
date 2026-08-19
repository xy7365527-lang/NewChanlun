# #1111 DevSkim 11,445 findings 可审计基线（2026-08-20）

> 票：[#1111](https://github.com/xy7365527-lang/NewChanlun/issues/1111)
> 基线主线：`af10903417cb6ad84b066733636a784489a49ab2`
> 真实运行：[DevSkim run 32305956877](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32305956877)
> 原始证据：[artifact 9384722211](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32305956877/artifacts/9384722211)（`devskim-sarif`）

## 0. 结论先行

真实 artifact 的 **11,445** 个 SARIF result 已逐条机械展开，并按路径 strata 审核：

- **11,170（97.597%）生成物/fixture/生成 manifest**；
- **7（0.061%）vendored/第三方**；
- **8（0.070%）本仓真实安全问题**；
- **260（2.272%）本仓维护文件中的明确误报**。

四桶严格账平：`11,170 + 7 + 8 + 260 = 11,445`。真实问题分三类开了 [#1116](https://github.com/xy7365527-lang/NewChanlun/issues/1116)、[#1117](https://github.com/xy7365527-lang/NewChanlun/issues/1117)、[#1118](https://github.com/xy7365527-lang/NewChanlun/issues/1118)，本票没有顺手改业务。

本票只把有正面证据的 **11,177** 条生成/fixture/第三方结果从扫描输入移除；重跑剩 **268 = 260 明确误报 + 8 真问题**。严格 gate 仍以 268 非零退出 1，**零阈值没有降低、没有 rule allowlist、没有 `continue-on-error`、没有静默 skip**。260 条误报没有粗暴按整条 rule 或整个维护目录排掉，因为那会遮蔽未来同规则的真实回归。

## 1. 上游口径与证据链

### 1.1 已读的裁定链

1. `.chanlun/review-results/issue1106-code-scanning-entitlement-20260819.md`：个人 Pro 私有仓没有 private Code scanning entitlement；DevSkim 可保留 ordinary artifact + 本地 findings gate，artifact 不得冒充 Security tab。
2. #1107 Resolution A：scanner/解析失败、SARIF 缺失或**任一 finding** 都红；只允许零 finding 绿。
3. #1108 最终验收评论：run 32305956877 的 scanner 与 ordinary artifact upload 成功，严格 gate 因 11,445 findings 正确失败；artifact 9384722211 可下载，分诊由 #1111 承接。

### 1.2 不可变基线

| 项 | 固定值 | 一手证据 |
| --- | --- | --- |
| 主线 SHA | `af10903417cb6ad84b066733636a784489a49ab2` | run API 的 `headSha`；checkout log 第 186/256 行 |
| workflow / job | `DevSkim` / job `96238652519` | run 32305956877 API |
| action 输入引用 | `microsoft/DevSkim-Action@v1` | 基线 workflow 与 run log 第 257 行 |
| action 实际 commit | `4b5047945a44163b94642a1cecc0d93a3f428cc6` | run log 第 30 行 |
| action commit 上游 | 2025-05-21 `Fix archive crawling (#32)` | [上游 commit](https://github.com/microsoft/DevSkim-Action/commit/4b5047945a44163b94642a1cecc0d93a3f428cc6) |
| DevSkim CLI package | `Microsoft.CST.DevSkim.Cli 1.0.90` | Docker build log 第 109-112 行 |
| SARIF driver version | `1.0.90+fb2d676ce4` | `runs[0].tool.driver.version` |
| Docker base | `mcr.microsoft.com/dotnet/sdk:8.0@sha256:306301580fcaa5b445180e759db59309979002d1000669cb4cf58a567d0014bc` | run log 第 52 行；本地复建同 digest |
| 原始 action ignore | `**/.git/**,**/bin/**` | run log 第 263/265 行 |
| archive zip | 590,245 bytes；sha256 `afcbee9d096f39bea1f0f166bb0b7d0c876b0090c14089eccd5d188f5cd439d1` | upload log 第 284-289 行 + 本地下载复核 |
| `devskim-results.sarif` | 8,167,562 bytes；sha256 `a3014a6990182e11b7e38540c0405937b58d1b5c718d4615f36d4530b19a3351` | artifact 解包后 `shasum -a 256` |
| SARIF 结构 | 2.1.0；1 run；11,445 results；8 rules；127 paths | 独立 Python/jq 解析 |

原始 action commit 的 `Dockerfile` 有两处**都不**固定 scanner runtime：base image 是 mutable tag `mcr.microsoft.com/dotnet/sdk:8.0`（无 digest），CLI 装法是 `dotnet tool install ... Microsoft.CST.DevSkim.Cli`（**无 `--version`**）。因此“action commit 固定”只固定了 wrapper，**不等于**扫描器 runtime 固定——同一 action commit 在不同时间 build 出的 CLI/base 可以不同。

本票不再依赖上游 action 的 mutable build，改为**自管最小 scanner runtime**（`.github/devskim/Dockerfile` + `entrypoint.sh`），两轴都钉死：

- base image 用 immutable digest `mcr.microsoft.com/dotnet/sdk:8.0@sha256:306301580fcaa5b445180e759db59309979002d1000669cb4cf58a567d0014bc`（与真实 run 32305956877 同 digest；该 digest 是多架构 manifest list，`ubuntu-latest`=amd64 确定解到 amd64 变体）；
- CLI 用 `dotnet tool install --version 1.0.90 --add-source https://api.nuget.org/v3/index.json Microsoft.CST.DevSkim.Cli`（NuGet 版本发布后不可变，`--version` + 官方 feed 唯一确定 CLI 内容，对应 SARIF driver `1.0.90+fb2d676ce4`）；
- `entrypoint.sh` 逐字复刻上游默认输入下的 analyze 命令（含 `set -o noglob`），只接 `source-dir / output-file / ignore-globs` 三参；
- workflow 用 `docker build .github/devskim` + `docker run` 挂载 `$GITHUB_WORKSPACE`，不再 `uses: microsoft/DevSkim-Action@…`。

gate 侧同时对 SARIF driver 断言 **identity（`name` 必须 `devskim`）与 `version`（`1.0.90+fb2d676ce4`）**：`--expected-tool-name 'devskim'` + `--expected-tool-version '1.0.90+fb2d676ce4'`，任一漂移 fail-loud 并写入 summary，而不是静默接受一份不可比较或来源不明的新基线。自管 runtime 在提交树上跑出的 residual 仍是 **268**，canonical residual digest 仍是 `5da677c85845dea66cd47a96525c9d5ee4112e82534e30a8d0d81fbf6ad68abe`，与上游 action 逐结果一致，证明这次替换是行为等价的、只是把 runtime 从 mutable 变成可复现。

### 1.3 实际命令

上游 `entrypoint.sh` 在本次默认输入下展开为：

```bash
/tools/devskim analyze \
  --source-code "$GITHUB_WORKSPACE" \
  --output-file "$GITHUB_WORKSPACE/devskim-results.sarif" \
  --ignore-globs '**/.git/**,**/bin/**'
```

`should-scan-archives=false`；`exclude-rules`、`options-json`、`extra-options` 均为空。自管 `entrypoint.sh` 在默认三参下展开的命令与此逐字一致（仅 `--source-code`/`--output-file` 路径为容器内 `/workspace`）。报告中的本地对拍用**自管 runtime**（同 base digest、同 CLI 1.0.90/driver `1.0.90+fb2d676ce4`）与同一主线树；与真实 run 的原始上游 action 输出逐结果对拍一致。

## 2. 机械统计

### 2.1 ruleId × level

| ruleId | 规则名 | error | warning | note | 合计 | 路径数 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `DS173237` | DoNotStoreTokensOrKeysInSourceCode | 11187 | 0 | 0 | 11187 | 19 |
| `DS162092` | DoNotLeaveDebugCodeInProduction | 0 | 0 | 92 | 92 | 46 |
| `DS176209` | SuspiciousComment | 0 | 0 | 61 | 61 | 29 |
| `DS126858` | WeakbrokenHashAlgorithm | 45 | 0 | 0 | 45 | 6 |
| `DS148264` | DoNotUseWeaknoncryptographicRandomNumberGenerators | 34 | 0 | 0 | 34 | 20 |
| `DS137138` | InsecureUrl | 0 | 19 | 0 | 19 | 12 |
| `DS172411` | ReviewSettimeoutForUntrustedData | 0 | 0 | 6 | 6 | 6 |
| `DS425000` | DoNotDeserializeUntrustedData | 0 | 0 | 1 | 1 | 1 |

总 level 分布：**error 11,266 / warning 19 / note 160 = 11,445**。这里的 `level` 是 SARIF 产出级别，不等于本报告人工裁决；例如 `DS173237` 的 11,187 个 `error` 中 11,163 个位于生成物/fixture，剩余 24 个也是公开 hash/commit fixture，并非 11,187 份凭据泄露。

### 2.2 顶层路径桶 × ruleId

| 顶层桶 | `DS173237` | `DS162092` | `DS176209` | `DS126858` | `DS148264` | `DS137138` | `DS172411` | `DS425000` | 合计 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `.chanlun` | 8707 | 0 | 10 | 0 | 0 | 0 | 0 | 0 | 8717 |
| `topological-computation` | 2395 | 29 | 1 | 0 | 8 | 8 | 3 | 1 | 2445 |
| `scripts` | 20 | 36 | 3 | 0 | 2 | 8 | 0 | 0 | 69 |
| `skills-lock.json` | 54 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 54 |
| `analysis` | 0 | 4 | 1 | 17 | 10 | 0 | 0 | 0 | 32 |
| `chanlun` | 6 | 0 | 0 | 26 | 0 | 0 | 0 | 0 | 32 |
| `rust` | 2 | 0 | 26 | 0 | 0 | 0 | 0 | 0 | 28 |
| `trading_system` | 0 | 2 | 17 | 0 | 0 | 0 | 0 | 0 | 19 |
| `tests` | 0 | 0 | 0 | 2 | 11 | 3 | 0 | 0 | 16 |
| `src` | 0 | 6 | 1 | 0 | 0 | 0 | 0 | 0 | 7 |
| `.claude` | 0 | 5 | 0 | 0 | 0 | 0 | 1 | 0 | 6 |
| `tmp` | 1 | 0 | 2 | 0 | 3 | 0 | 0 | 0 | 6 |
| `frontend` | 0 | 3 | 0 | 0 | 0 | 0 | 2 | 0 | 5 |
| `.sandcastle` | 0 | 4 | 0 | 0 | 0 | 0 | 0 | 0 | 4 |
| `tws_margin_check.py` | 0 | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |
| `prototypes` | 2 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 2 |
| `.agents` | 0 | 1 | 0 | 0 | 0 | 0 | 0 | 0 | 1 |

### 2.3 代码所有权/处置 × ruleId

| 分类 | `DS173237` | `DS162092` | `DS176209` | `DS126858` | `DS148264` | `DS137138` | `DS172411` | `DS425000` | 合计 | 占比 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 生成物/fixture | 11163 | 0 | 4 | 0 | 3 | 0 | 0 | 0 | 11170 | 97.597% |
| vendored/第三方 | 0 | 6 | 0 | 0 | 0 | 0 | 1 | 0 | 7 | 0.061% |
| 本仓真问题 | 0 | 0 | 0 | 0 | 0 | 7 | 0 | 1 | 8 | 0.070% |
| 明确误报 | 24 | 86 | 57 | 45 | 31 | 12 | 5 | 0 | 260 | 2.272% |

按 level 再账平：

| 分类 | error | warning | note | 合计 |
| --- | ---: | ---: | ---: | ---: |
| 生成物/fixture | 11166 | 0 | 4 | 11170 |
| vendored/第三方 | 0 | 0 | 7 | 7 |
| 本仓真问题 | 0 | 7 | 1 | 8 |
| 明确误报 | 100 | 12 | 148 | 260 |

分类优先级固定为：`生成物/fixture → vendored/第三方 → 本仓真问题 → 明确误报`。只有两个路径同时包含不同处置：`topological-computation/feeds/arxiv.py`（网络请求真问题 + XML namespace 误报）与 `topological-computation/frontend/src/tokens.ts`（公网 HTTP 真问题 + localhost 误报）；附录按 result 逐项拆开，未整文件偷换分类。

## 3. 分层人工核验

方法：先对 11,445 条建立 `(ruleId, level, path, line, column, snippet)` 表；除 `DS173237` 外，**其余 258 条逐条打开源行**。`DS173237` 按 19 个路径分层：对 11 个生成/fixture strata 取首/中/末共 31 个样本并核生产者/schema，同时**全读剩余 24 个本仓维护文件命中**。这不是随机“看几个差不多”，而是高频生成 strata + 全量维护代码的风险导向分层。

| ruleId | 核验范围 | 结果与判据 |
| --- | --- | --- |
| `DS173237`（11,187） | 19 paths；生成 strata 31 个首/中/末样本；24 个维护代码命中全量 | 11,163 条是 block/content/session SHA、实验 position id、skill `computedHash`、golden commit anchor 等生成/fixture 数据；24 条是 SHA-256 已知向量、Git SHA、block hash 测试/迁移常量。零条是 token/key。 |
| `DS162092`（92） | 92/92 | 86 条本仓命中全是 `localhost`/`127.0.0.1` 的显式 loopback bind、TWS/CDP/IPFS/本地 daemon 或开发代理；6 条来自第三方 skill。规则只凭字面猜“debug code”，未发现非 loopback 暴露。全部不是安全问题。 |
| `DS176209`（61） | 61/61 | 57 条维护文件命中是诚实 TODO/阶段缺口/扫描词本身，4 条在生成/fixture；它们可代表功能 backlog，但不是凭据、注入或访问控制问题，不能借 DevSkim 安全票批量改业务。 |
| `DS126858`（45） | 45/45 | 43 条 `MD5/md5` 只用于历史 dump 字节一致性/诊断摘要，不承担口令、签名、认证或完整性安全边界；2 条是变量名 `md2` 被子串误击。45 条均明确误报。 |
| `DS148264`（34） | 34/34 | 31 条在统计实验、可复现 property/test、算法遍历；3 条在 `tmp/`。均为固定 seed / shuffle 的模拟用途，无 token/session/cryptographic selection。 |
| `DS137138`（19） | 19/19 | **7 真**：FengLiang 公网 daemon HTTP 默认/示例 6 条 + arXiv HTTP 首跳 1 条。**12 误报**：本机 CDP/代理 8、测试占位 `http://x` 3、W3C Atom namespace 1。 |
| `DS172411`（6） | 6/6 | 5 条本仓代码与 1 条第三方 skill 都传函数 callback，不把字符串/不可信输入交给 `setTimeout` 执行；规则仅凭 API 名命中。 |
| `DS425000`（1） | 1/1 | **真问题**：`topological-computation/snet_cache.py:272` 从用户 HOME cache 直接 `pickle.load`，加载前无真实性/完整性校验；异常回退发生在潜在 payload 执行之后。 |

### 3.1 三类真实安全问题

| 下游票 | SARIF 归属 | 数量 | 事实 |
| --- | --- | ---: | --- |
| [#1116 S_net cache 移除未校验 pickle](https://github.com/xy7365527-lang/NewChanlun/issues/1116) | `DS425000/note` | 1 | `snet_cache.py:272`；仓内 `.chanlun/genealogy/settled/456-...md:56-84` 已裁“放弃 pickle → JSONL”，但实施未落地。 |
| [#1117 FengLiang 公网 daemon 默认链改 TLS](https://github.com/xy7365527-lang/NewChanlun/issues/1117) | `DS137138/warning` | 6 | `frontend/src/tokens.ts:37,56,62,68`、`hooks/useStore.ts:440`、`components/InstanceManager.tsx:202`；同链还有 DevSkim 未报的 `ws://`。 |
| [#1118 arXiv API 首跳改 HTTPS](https://github.com/xy7365527-lang/NewChanlun/issues/1118) | `DS137138/warning` | 1 | `feeds/arxiv.py:14`；只读探针确认 HTTP 返回 301，直接 HTTPS 同查询返回 `200 application/atom+xml`。 |

每类开票前都执行两路查重：tracker 用 `gh issue list --state all`（包含 closed），仓内用 `grep -RIn` 扫 `analysis/ docs/ .chanlun/`。#1117/#1118 两路均无既有承接；#1116 tracker 无票，但推论层命中 #456 的既定“pickle→JSONL”裁定，所以新票明确是**实施既有裁定**而不是另起一份决策。

## 4. 版本控制内的扫描排除

### 4.1 排除清单与正面证据

| ignore-glob | 移除数 | 规则分布 | 正面证据 |
| --- | ---: | --- | --- |
| `**/.chanlun/block-topology/concept_registry.json` | 3703 | DS173237:3702,DS176209:1 | `build_concept_registry.py:8,22,200-213` 声明并写出反向索引 |
| `**/.chanlun/block-topology/morse_landscape.json` | 2272 | DS173237:2272 | `build_morse_landscape.py:8,20,160-186` 声明并写出派生地形 |
| `**/.chanlun/block-topology/meta.json` | 633 | DS173237:633 | `scripts/block_topology.py:490-496` 写 topology metadata；命中全在 hash/id mapping |
| `**/.chanlun/concept_registry.json` | 2101 | DS173237:2100,DS176209:1 | `scripts/concept_registry.py:61,64-74,136-146` 全量重算并导出 |
| `**/topological-computation/experiment_long_run_2000.json` | 1125 | DS173237:1125 | `experiment_long_run.py:317-320` 写出的实验结果 |
| `**/topological-computation/experiment_real_result.json` | 479 | DS173237:479 | `experiment_real.py:571-593` 写出的实验结果 |
| `**/topological-computation/experiment_f_criterion_2000.json` | 271 | DS173237:271 | `experiment_f_criterion.py:829-832` 写出的实验结果 |
| `**/topological-computation/signifier_net/.dialogue_ingest_state.json` | 520 | DS173237:520 | `dialogue_ingest.py:12,82-89` 写 session SHA-256 去重状态 |
| `**/skills-lock.json` | 54 | DS173237:54 | skills CLI 的 GitHub source/computedHash lock；命中全是 `computedHash` |
| `**/chanlun/review-results/treasury-reverify-t1-armR-trades-golden-20260727.json` | 6 | DS173237:6 | `scripts/check_armR_trades_digest.py:56` 明定 GOLDEN；命中为提交锚 |
| `**/tmp/**` | 6 | DS148264:3,DS173237:1,DS176209:2 | 根 `.gitignore:54` 明定 Data/temp；旧 tracked 残留不属维护代码 |
| `**/.claude/skills/brainstorming/**` | 5 | DS162092:4,DS172411:1 | 与 `obra/superpowers@6efe32c9` 对应文件逐字节相同 |
| `**/.agents/skills/diagnosing-bugs/**` | 1 | DS162092:1 | `skills-lock.json` 明记 `mattpocock/skills` GitHub 来源 |
| `**/.claude/skills/diagnosing-bugs/**` | 1 | DS162092:1 | symlink 投影到上一条第三方 skill；扫描器重复遍历 |

共移除 **11,177 = 11,170 生成/fixture + 7 vendored/第三方**。每个 pattern 都逐字落在 `.github/workflows/devskim.yml`；没有隐藏环境变量或 Actions UI 配置。

关键边界：

- 没有 `exclude-rules`，没有按 `DS173237` 等规则整条放行；
- 没有排除 `analysis/`、`scripts/`、`rust/`、`topological-computation/` 等维护目录；
- `.sandcastle/image/skills/serena/**` 等本仓维护的工具代码仍在扫描；
- 260 条明确误报仍保留，避免用粗路径/整规则排除遮蔽未来真正的 secret、MD5 安全用途、远端 HTTP 或字符串 `setTimeout`；
- `scripts/devskim_sarif_gate.sh` 的 `finding_count > 0` 失败逻辑不变，只新增 SARIF driver **identity（`name`=`devskim`）与 `version`（`1.0.90+fb2d676ce4`）** 的双重 fail-loud 契约（`--expected-tool-name` + `--expected-tool-version`）。

### 4.2 第三方证据强度

- `.claude/skills/brainstorming/{SKILL.md,scripts/helper.js,scripts/start-server.sh}` 与 `obra/superpowers@6efe32c9e2dd002d0c394e861e0529675d1ab32e` 对应文件逐字节相同；三个 sha256 分别为 `bba47904a7f6bbee3bf8a107ebbe84e65d392be683bbb898ded736b29e415f90`、`e763d82f32b4ebb320be2f0d08449d3e5a0585778edf5c407c1fdab3f39976a5`、`3442a54b5ee5511637d27bf696e092b72f94c2324f40b5ba4410da4dd0c23bc8`。
- `.agents/skills/diagnosing-bugs` 的 `skills-lock.json` 条目明记 `sourceType=github`、`source=mattpocock/skills`、`computedHash=fd6c99466b7ba43be624e6e66ed6d7af2796ded7218f24c83820823977819e22`；`.claude/skills/diagnosing-bugs` 是指向它的 symlink projection，原始扫描因此把同一 localhost 模板报了两次。

## 5. 重跑与差异解释

### 5.1 无新排除的复现

用**自管 scanner runtime**（base image immutable digest `sha256:306301580f…` 与真实 run 完全相同，CLI 固定 `--version 1.0.90` / driver 1.0.90+fb2d676ce4）复建容器；在 `af109034...` 全树只忽略原始 `.git/bin`：

- 真实 artifact：11,445 results，raw SARIF sha256 `a3014a6990182e11b7e38540c0405937b58d1b5c718d4615f36d4530b19a3351`；
- 本地 full rerun：11,445 results，raw SARIF sha256 `4fb173bcdda4c0a46d10c85beb8a4f08c7c9afb841c68c7894f8f2b2c8ca51c5`；
- 两者的完整 result multiset **逐 JSON 对象相同**；差异只有 `results` 数组顺序。

为避免把并行遍历顺序当内容漂移，另算 canonical result digest：每个 result 用 JSON key 排序、紧凑编码，全部 result 字符串排序，以 `\n` 连接并保留末尾换行，再做 SHA-256。两边均为：

`1d4b013b6a4a8fe38d79483fd54d7737ac3d328ea489976fe4ad211cfdcc1402`

因此 raw SHA 不同是**顺序差异**，不是 finding 差异。

### 5.2 排除后的对拍

固定 exclusions 重跑得到 **268**：

| ruleId | error | warning | note | 合计 |
| --- | ---: | ---: | ---: | ---: |
| `DS126858` | 45 | 0 | 0 | 45 |
| `DS137138` | 0 | 19 | 0 | 19 |
| `DS148264` | 31 | 0 | 0 | 31 |
| `DS162092` | 0 | 0 | 86 | 86 |
| `DS172411` | 0 | 0 | 5 | 5 |
| `DS173237` | 24 | 0 | 0 | 24 |
| `DS176209` | 0 | 0 | 57 | 57 |
| `DS425000` | 0 | 0 | 1 | 1 |
| **合计** | **100** | **19** | **149** | **268** |

`11,445 - 11,177 = 268`，且 residual result multiset 与原 artifact 中剔除上述路径后的集合逐对象一致。candidate 的全部改动文件（`.github/workflows/devskim.yml`、`.github/devskim/Dockerfile`、`.github/devskim/entrypoint.sh`、`scripts/devskim_sarif_gate.sh`、本报告）各自新增 finding 均为 0——`.github/` 不在排除清单内、被正常扫描，Dockerfile 里的 base image digest 未触发 `DS173237`。用**自管 runtime** 对完整提交树 fresh 重扫仍为 268，canonical residual digest 不变。排除后多次 rerun 的 raw SARIF SHA 会随结果顺序变化（前两次为 `4710b3af2187eb3a30360f012932b60c6d3f4355decf9db44ce10059451bfd37` / `3cc926101e90817fef58f647ab6587d74655ade8e554de2114e3efb39e260eb3`），canonical residual digest 均为：

`5da677c85845dea66cd47a96525c9d5ee4112e82534e30a8d0d81fbf6ad68abe`

对 residual 运行仓内 gate：driver name=`devskim`、version 与期望相同，findings=`268`，exit=`1`；failure detail 仍为 “this gate requires zero”。这正是 #1107 A 的预期红，不是需要修掉的 CI 噪声。

## 6. 靶向验证

本报告落盘前执行：

- `bash -n scripts/devskim_sarif_gate.sh` 与 `bash -n .github/devskim/entrypoint.sh`；
- ShellCheck 0.11.0（gate + entrypoint 均 clean）；
- actionlint 1.7.12（workflow clean）；
- Ruby `YAML.load_file`（workflow 可解析）；
- `docker build .github/devskim`（自管镜像成功 build，装出 CLI `1.0.90+fb2d676ce4`）；
- gate fixture 全矩阵：正确 name+version+zero→0；正确 name+version+nonzero→1；错误 version+zero→1；缺 version→1；**Other-name 同版本 zero→1**（driver `name` 非 `devskim` 时即使 version 对也拒绝）；缺 name→1；
- 真实 residual SARIF：name=`devskim`、version match、268 findings、gate→1；
- Docker 对拍（自管 runtime）：full 11,445（canonical multiset `1d4b0136…`）；排除后 268（canonical residual `5da677c8…`）；提交树含全部改动文件仍 268、digest 不变；`.github/devskim/` 隔离扫描新增 finding=0；
- `git diff --check`。

这些是靶向验证，不声称重跑全仓 Rust/Python/Lean 测试；本票没有改业务代码。

## 附录 A：完整 path × ruleId × level × 分类

下表是 SARIF 的 11,445 个 result 按四列机械 group 后的完整 140 行，不省略路径。

| 路径 | ruleId | level | 分类 | 数量 |
| --- | --- | --- | --- | ---: |
| `.agents/skills/diagnosing-bugs/scripts/hitl-loop.template.sh` | `DS162092` | note | vendored/第三方 | 1 |
| `.chanlun/archive/pre-rtas-goal-arch-20260627/ceremony_scan.py` | `DS176209` | note | 明确误报 | 1 |
| `.chanlun/archive/pre-rtas-goal-arch-20260627/dispatch-dag.yaml` | `DS176209` | note | 明确误报 | 2 |
| `.chanlun/block-topology/concept_registry.json` | `DS173237` | error | 生成物/fixture | 3702 |
| `.chanlun/block-topology/concept_registry.json` | `DS176209` | note | 生成物/fixture | 1 |
| `.chanlun/block-topology/meta.json` | `DS173237` | error | 生成物/fixture | 633 |
| `.chanlun/block-topology/morse_landscape.json` | `DS173237` | error | 生成物/fixture | 2272 |
| `.chanlun/concept_registry.json` | `DS173237` | error | 生成物/fixture | 2100 |
| `.chanlun/concept_registry.json` | `DS176209` | note | 生成物/fixture | 1 |
| `.chanlun/dispatch-dag.yaml` | `DS176209` | note | 明确误报 | 2 |
| `.chanlun/dispatch-spec.yaml` | `DS176209` | note | 明确误报 | 2 |
| `.chanlun/review-results/.kappa-ruling-output-20260704.txt` | `DS176209` | note | 明确误报 | 1 |
| `.claude/skills/brainstorming/scripts/helper.js` | `DS172411` | note | vendored/第三方 | 1 |
| `.claude/skills/brainstorming/scripts/start-server.sh` | `DS162092` | note | vendored/第三方 | 4 |
| `.claude/skills/diagnosing-bugs/scripts/hitl-loop.template.sh` | `DS162092` | note | vendored/第三方 | 1 |
| `.sandcastle/image/prime-agent-settings.json` | `DS162092` | note | 明确误报 | 1 |
| `.sandcastle/image/skills/serena/src/serena/__init__.py` | `DS162092` | note | 明确误报 | 3 |
| `analysis/_c_seg_fix_capture.py` | `DS126858` | error | 明确误报 | 17 |
| `analysis/_tencent_warrants_discover.py` | `DS162092` | note | 明确误报 | 1 |
| `analysis/_tencent_warrants_quotes.py` | `DS162092` | note | 明确误报 | 1 |
| `analysis/_tw_q4_eff_gearing.py` | `DS162092` | note | 明确误报 | 1 |
| `analysis/buy_28316_warrant.py` | `DS162092` | note | 明确误报 | 1 |
| `analysis/gamma_delta_1m_resolution.py` | `DS148264` | error | 明确误报 | 2 |
| `analysis/gamma_delta_1min_discriminant_search.py` | `DS148264` | error | 明确误报 | 1 |
| `analysis/gamma_delta_discriminant_search_v2.py` | `DS148264` | error | 明确误报 | 1 |
| `analysis/gamma_delta_full_verification.py` | `DS148264` | error | 明确误报 | 2 |
| `analysis/p3_random_gate_control.py` | `DS148264` | error | 明确误报 | 2 |
| `analysis/p3_random_gate_control.py` | `DS176209` | note | 明确误报 | 1 |
| `analysis/verify_holonomy_full_null.py` | `DS148264` | error | 明确误报 | 2 |
| `chanlun/review-results/colocation-chain-analyze-20260724.py` | `DS126858` | error | 明确误报 | 1 |
| `chanlun/review-results/endorsement-failure-instrument-verify-20260724.py` | `DS126858` | error | 明确误报 | 6 |
| `chanlun/review-results/owner-attribution-fix-verify-20260724.py` | `DS126858` | error | 明确误报 | 9 |
| `chanlun/review-results/treasury-reverify-t1-armR-trades-golden-20260727.json` | `DS173237` | error | 生成物/fixture | 6 |
| `chanlun/review-results/typed-none-strict-chain-analyze-20260723.py` | `DS126858` | error | 明确误报 | 10 |
| `frontend/src/hooks/useOverlay.ts` | `DS172411` | note | 明确误报 | 1 |
| `frontend/src/hooks/useSearch.ts` | `DS172411` | note | 明确误报 | 1 |
| `frontend/vite.config.ts` | `DS162092` | note | 明确误报 | 3 |
| `prototypes/task68-ledger-adopt/tests/properties.rs` | `DS173237` | error | 明确误报 | 1 |
| `prototypes/task72-gap-connector/tests/properties.rs` | `DS173237` | error | 明确误报 | 1 |
| `rust/src/theta_v0/classifier/consume_at.rs` | `DS176209` | note | 明确误报 | 6 |
| `rust/src/theta_v0/nautilus/account_adapter.rs` | `DS176209` | note | 明确误报 | 4 |
| `rust/src/theta_v0/nautilus/bar_adapter.rs` | `DS176209` | note | 明确误报 | 5 |
| `rust/src/theta_v0/nautilus/order_adapter.rs` | `DS176209` | note | 明确误报 | 4 |
| `rust/src/theta_v0/nautilus/strategy.rs` | `DS176209` | note | 明确误报 | 7 |
| `rust/src/theta_v0/venue_fee/datum_io.rs` | `DS173237` | error | 明确误报 | 2 |
| `scripts/_probe_ohlcv.py` | `DS137138` | warning | 明确误报 | 2 |
| `scripts/_probe_ohlcv.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/add_263_relations.py` | `DS173237` | error | 明确误报 | 4 |
| `scripts/anthropic_thinking_sanitizer_proxy.py` | `DS137138` | warning | 明确误报 | 1 |
| `scripts/anthropic_thinking_sanitizer_proxy.py` | `DS162092` | note | 明确误报 | 7 |
| `scripts/async_self_reference.py` | `DS176209` | note | 明确误报 | 2 |
| `scripts/cdp_extract_labels.py` | `DS137138` | warning | 明确误报 | 1 |
| `scripts/cdp_extract_labels.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/cdp_extract_multiframe.py` | `DS137138` | warning | 明确误报 | 1 |
| `scripts/cdp_extract_multiframe.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/cdp_extract_ohlcv.py` | `DS137138` | warning | 明确误报 | 1 |
| `scripts/cdp_extract_ohlcv.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/ceremony_scan.py` | `DS176209` | note | 明确误报 | 1 |
| `scripts/dev.ps1` | `DS162092` | note | 明确误报 | 4 |
| `scripts/hk_index_cdp_fetch.py` | `DS137138` | warning | 明确误报 | 1 |
| `scripts/hk_index_cdp_fetch.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/ipfs_deploy.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/openclaw_scheduler.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/optimal_morse.py` | `DS148264` | error | 明确误报 | 2 |
| `scripts/realtime_server.py` | `DS162092` | note | 明确误报 | 2 |
| `scripts/start_claude_via_sanitizer.ps1` | `DS137138` | warning | 明确误报 | 1 |
| `scripts/start_claude_via_sanitizer.ps1` | `DS162092` | note | 明确误报 | 2 |
| `scripts/start_with_gateway.sh` | `DS162092` | note | 明确误报 | 11 |
| `scripts/tests/test_check_armR_trades_digest.py` | `DS173237` | error | 明确误报 | 7 |
| `scripts/tests/test_interrupt_materializer.py` | `DS173237` | error | 明确误报 | 7 |
| `scripts/tws/fetch_1m_tws.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/tws/fetch_30m_tws.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/tws_fetch_hk700_1m.py` | `DS162092` | note | 明确误报 | 1 |
| `scripts/update_meta_263.py` | `DS173237` | error | 明确误报 | 1 |
| `scripts/verify_263.py` | `DS173237` | error | 明确误报 | 1 |
| `skills-lock.json` | `DS173237` | error | 生成物/fixture | 54 |
| `src/newchan/b_chart.py` | `DS176209` | note | 明确误报 | 1 |
| `src/newchan/config.py` | `DS162092` | note | 明确误报 | 1 |
| `src/newchan/gateway.py` | `DS162092` | note | 明确误报 | 2 |
| `src/newchan/server.py` | `DS162092` | note | 明确误报 | 3 |
| `tests/test_buypoint_score.py` | `DS148264` | error | 明确误报 | 1 |
| `tests/test_codex_challenger.py` | `DS126858` | error | 明确误报 | 2 |
| `tests/test_dual_merge_tree.py` | `DS148264` | error | 明确误报 | 1 |
| `tests/test_fetch_massive_tick.py` | `DS137138` | warning | 明确误报 | 3 |
| `tests/test_online_persistence.py` | `DS148264` | error | 明确误报 | 5 |
| `tests/test_ph_persistence_closed_form.py` | `DS148264` | error | 明确误报 | 2 |
| `tests/test_settle_trigger.py` | `DS148264` | error | 明确误报 | 2 |
| `tmp/fold-intrinsic-experiment/cross_compare.py` | `DS148264` | error | 生成物/fixture | 1 |
| `tmp/pdf_strategic_dialogue_v1.txt` | `DS176209` | note | 生成物/fixture | 1 |
| `tmp/pdf_strategic_dialogue_v2_full.txt` | `DS176209` | note | 生成物/fixture | 1 |
| `tmp/permutation_test_negates.py` | `DS148264` | error | 生成物/fixture | 2 |
| `tmp/update_meta.py` | `DS173237` | error | 生成物/fixture | 1 |
| `topological-computation/ceremony.py` | `DS162092` | note | 明确误报 | 4 |
| `topological-computation/chain/ipfs_client.py` | `DS162092` | note | 明确误报 | 2 |
| `topological-computation/chain/setup_private.sh` | `DS162092` | note | 明确误报 | 2 |
| `topological-computation/daemon_loop.py` | `DS162092` | note | 明确误报 | 1 |
| `topological-computation/daemon_multiproc.py` | `DS162092` | note | 明确误报 | 2 |
| `topological-computation/daemon_server.py` | `DS162092` | note | 明确误报 | 2 |
| `topological-computation/deploy.bat` | `DS162092` | note | 明确误报 | 1 |
| `topological-computation/deploy.py` | `DS162092` | note | 明确误报 | 1 |
| `topological-computation/deploy.sh` | `DS162092` | note | 明确误报 | 1 |
| `topological-computation/engine.py` | `DS176209` | note | 明确误报 | 1 |
| `topological-computation/experiment_f_criterion.py` | `DS148264` | error | 明确误报 | 1 |
| `topological-computation/experiment_f_criterion_2000.json` | `DS173237` | error | 生成物/fixture | 271 |
| `topological-computation/experiment_long_run_2000.json` | `DS173237` | error | 生成物/fixture | 1125 |
| `topological-computation/experiment_real_result.json` | `DS173237` | error | 生成物/fixture | 479 |
| `topological-computation/feeds/arxiv.py` | `DS137138` | warning | 明确误报 | 1 |
| `topological-computation/feeds/arxiv.py` | `DS137138` | warning | 本仓真问题 | 1 |
| `topological-computation/frontend/src/components/InstanceManager.tsx` | `DS137138` | warning | 本仓真问题 | 1 |
| `topological-computation/frontend/src/hooks/useDaemonWS.ts` | `DS172411` | note | 明确误报 | 1 |
| `topological-computation/frontend/src/hooks/useMultiDaemon.ts` | `DS172411` | note | 明确误报 | 1 |
| `topological-computation/frontend/src/hooks/useStore.ts` | `DS137138` | warning | 本仓真问题 | 1 |
| `topological-computation/frontend/src/src/hooks/useDaemonWS.ts` | `DS172411` | note | 明确误报 | 1 |
| `topological-computation/frontend/src/src/tokens.ts` | `DS162092` | note | 明确误报 | 2 |
| `topological-computation/frontend/src/src/views/GalaxyView.tsx` | `DS162092` | note | 明确误报 | 1 |
| `topological-computation/frontend/src/tokens.ts` | `DS137138` | warning | 本仓真问题 | 4 |
| `topological-computation/frontend/src/tokens.ts` | `DS162092` | note | 明确误报 | 2 |
| `topological-computation/frontend/vite.config.ts` | `DS162092` | note | 明确误报 | 1 |
| `topological-computation/gateway_bridge.py` | `DS162092` | note | 明确误报 | 3 |
| `topological-computation/interactive.py` | `DS148264` | error | 明确误报 | 1 |
| `topological-computation/signifier_net/.dialogue_ingest_state.json` | `DS173237` | error | 生成物/fixture | 520 |
| `topological-computation/snet_cache.py` | `DS425000` | note | 本仓真问题 | 1 |
| `topological-computation/start_fengliang.py` | `DS162092` | note | 明确误报 | 3 |
| `topological-computation/swarm/test_swarm.py` | `DS162092` | note | 明确误报 | 1 |
| `topological-computation/traversal.py` | `DS148264` | error | 明确误报 | 1 |
| `topological-computation/verify_384_beta1_l2.py` | `DS148264` | error | 明确误报 | 1 |
| `topological-computation/verify_fg_corrected.py` | `DS148264` | error | 明确误报 | 2 |
| `topological-computation/verify_fg_predictions.py` | `DS148264` | error | 明确误报 | 2 |
| `trading_system/backtest_unn_stream.py` | `DS176209` | note | 明确误报 | 1 |
| `trading_system/config/broker_config.py` | `DS162092` | note | 明确误报 | 2 |
| `trading_system/config/broker_config.py` | `DS176209` | note | 明确误报 | 1 |
| `trading_system/config/instruments.py` | `DS176209` | note | 明确误报 | 2 |
| `trading_system/data/bar_aggregator.py` | `DS176209` | note | 明确误报 | 1 |
| `trading_system/execution/leverage_calculator.py` | `DS176209` | note | 明确误报 | 1 |
| `trading_system/execution/maker_optimizer.py` | `DS176209` | note | 明确误报 | 1 |
| `trading_system/live/runner.py` | `DS176209` | note | 明确误报 | 1 |
| `trading_system/persistence/trade_journal.py` | `DS176209` | note | 明确误报 | 1 |
| `trading_system/strategy/chanlun_strategy.py` | `DS176209` | note | 明确误报 | 5 |
| `trading_system/strategy/signal_bridge.py` | `DS176209` | note | 明确误报 | 3 |
| `tws_margin_check.py` | `DS162092` | note | 明确误报 | 2 |
