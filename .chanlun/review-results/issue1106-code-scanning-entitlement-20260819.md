# #1106 调研报告：私有仓 CodeQL / DevSkim 的授权根因与合规收敛

- 日期：2026-08-19
- 票据：[#1106](https://github.com/xy7365527-lang/NewChanlun/issues/1106)
- 仓库基线：`main @ ba449739ab0f5d648d7ab0076d94ed433600a244`
- 研究分支：`research/issue-1106-code-security`
- 边界：**只读探针**。未改计费、安全设置、workflow、远端分支；未调用写 API；未重跑 workflow。
- 证据口径：全文区分【本仓实测】、【GitHub 官方规则】、【据此推论】、【未执行】。外部事实只引 GitHub 官方文档、官方 API、GitHub 自有 action 清单与本仓 GitHub 页面/Actions 日志。

## 0. 裁决摘要

1. **报错的直接原因不是 `GITHUB_TOKEN` 权限，也不是 Actions 被禁用，而是服务端未给本仓启用 Code scanning。** 两个失败 run 的实际 token 都有 `SecurityEvents: write`；CodeQL 三个矩阵 job 都已完成分析并导出 SARIF，DevSkim scanner step 也成功，统一死在向 GitHub Code scanning 上传 SARIF。三个只读 Code scanning REST 端点均返回 HTTP 403 和同一句 `Code scanning is not enabled for this repository`。【本仓实测：[CodeQL run 32253705317](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32253705317)、[DevSkim run 32253705319](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32253705319)】
2. **当前拓扑没有可点开的合法开关。** 本仓是 `PRIVATE`，owner 的 GraphQL 类型是 `User`；当前账号公开页显示 `Pro`，且 viewer 的 organization 数为 0。GitHub 明写：Free/Pro 只能在 public repo 使用 Code scanning；private/internal repo 必须在 **GitHub Team 或 Enterprise 的 organization-owned repository** 上购买并启用 GitHub Code Security。因此当前这个“个人 Pro 名下的 private repo”不能仅靠某个 repository setting 或 `gh api` 获得 Code scanning。【GitHub 官方：[Private repository enablement](https://docs.github.com/en/code-security/reference/code-scanning/troubleshoot-analysis-errors/private-repository-enablement)、[GitHub plans](https://docs.github.com/en/get-started/learning-about-github/githubs-plans)、[当前账号公开页](https://github.com/xy7365527-lang)】
3. **非交互 API 本身存在，但只在先满足产品授权后有用。** `PATCH /repos/{owner}/{repo}` 接受 `security_and_analysis.code_security.status=enabled`；调用者须有 repo admin，或是组织 owner/security manager；fine-grained token / GitHub App 需 `Administration: write`。本账号当前确有 repo `ADMIN`，但权限不能绕过“个人所有 + Pro + private”的产品门。此 PATCH 本轮禁止且没有执行。【GitHub 官方：[Update a repository](https://docs.github.com/en/rest/repos/repos#update-a-repository)】
4. **启用不是零成本设置。** 先要 organization 的 Team/Enterprise 基础计划，再要 Code Security；GitHub 当前公开标价是 **$30 USD / active committer / month**，实际 license 用量按启用了产品的仓库中 90 天内的 unique active committers 计；Code scanning workflow 还消耗 private-repo Actions minutes。metered billing 下启用即可开始产生用量，volume/subscription 下须先买足 licenses。【GitHub 官方：[Advanced Security billing](https://docs.github.com/en/billing/concepts/product-billing/github-advanced-security)、[Buy Advanced Security](https://docs.github.com/en/billing/how-tos/products/buy-advanced-security)、[GitHub Security pricing](https://github.com/security/advanced-security)、[Code scanning](https://docs.github.com/en/code-security/concepts/code-scanning/code-scanning)】
5. **推荐的当前收敛不是让两个上传 step 永久红，也不是对失败 `continue-on-error`。**
   - **CodeQL：有意停用/移除其触发，等待仓库迁入合资格组织并购买 Code Security。** 虽然 `github/codeql-action/analyze` 技术上支持 `upload: never` 后存 artifact，但 GitHub 官方同时明写：CodeQL/CodeQL CLI 对 private repo 的许可入口也是 organization-owned Team/Enterprise + Code Security；CodeQL Terms 更明确禁止无付费客户许可时把软件用于 private repo/自动化 CI。故“继续扫个人 private repo、只是不上传”不是合规退场，不能推荐。【GitHub 官方：[About GitHub Advanced Security](https://docs.github.com/en/get-started/learning-about-github/about-github-advanced-security)、[CodeQL availability](https://docs.github.com/en/code-security/code-scanning/introduction-to-code-scanning/about-code-scanning-with-codeql)、[CodeQL Terms](https://github.com/github/codeql-cli-binaries/blob/main/LICENSE.md)、[官方 action 输入](https://github.com/github/codeql-action/blob/v4/analyze/action.yml)】
   - **DevSkim：可保留扫描，改为 SARIF artifact + 明确的本地 findings gate。** 当前 run 已证明 scanner 成功、只是 GitHub SARIF upload 失败。artifact 应 `if: always()` 留证，另一步按仓库裁定的 severity/allowlist 规则决定红绿；这样“扫描器故障/超阈值发现”才红，已知的产品缺席不再每次红。artifact 不能冒充 Security tab：它没有 GitHub alert 生命周期、PR annotations 和去重治理。【GitHub 官方：[Store and share workflow artifacts](https://docs.github.com/en/actions/tutorials/store-and-share-data)、[Uploading SARIF](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/integrate-with-existing-tools/upload-sarif-file)；本仓实测：[DevSkim run](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32253705319)】

## 1. 本仓现场：哪一层失败

### 1.1 账号、仓库与 Actions 设置

| 只读探针 | 2026-08-19 实测结果 | 能证明什么 |
|---|---|---|
| GraphQL `repository { isPrivate visibility owner { __typename } viewerPermission }` | `isPrivate=true`、`visibility=PRIVATE`、owner=`User(xy7365527-lang)`、viewer=`ADMIN` | 本仓不是 organization-owned；当前调用者有 repo admin |
| GraphQL `viewer { organizations { totalCount } }` | `0` | 当前账号没有可承载本仓的现成 organization |
| `GET /user` + 公开 profile | `/user` 因当前 OAuth token 无 `user` scope 而不返回私有 `plan` 字段（官方说明 classic PAT/OAuth token 需 `user` scope 才返回 private profile information）；公开 profile HTML 有 `title="Label: Pro"`【[Get the authenticated user](https://docs.github.com/en/rest/users/users#get-the-authenticated-user)】 | 当前个人计划实测为 Pro；不把缺失的 REST 字段误读为 Free |
| `GET /repos/xy7365527-lang/NewChanlun/actions/permissions` | HTTP 200，`enabled=true`、`allowed_actions=all` | Actions 总开关不是阻塞点 |
| `GET .../actions/permissions/workflow` | HTTP 200，默认 workflow permission=`read` | 仓库默认值是 read；但两个 workflow 已逐 job 显式申请所需写权限 |
| run 的 `GITHUB_TOKEN Permissions` | CodeQL 与 DevSkim 均为 `Actions: read`、`Contents: read`、`SecurityEvents: write` | 不是 `security-events: write` 漏配 |

GraphQL 是 GitHub 官方 API 的现场返回；计划 badge 来自账号自己的 GitHub 公开页。[GraphQL API 文档](https://docs.github.com/en/graphql)；[账号公开页](https://github.com/xy7365527-lang)。

`GET /repos/...` 的实际响应没有 `security_and_analysis` 字段。官方 REST 文档说 admin 可用该字段查看/更新受支持的安全功能，但**本报告不把字段缺失单独当授权判据**；决定性证据是下面三个专用端点的 403、官方产品门和 owner 类型。[Update a repository](https://docs.github.com/en/rest/repos/repos#update-a-repository)。

### 1.2 Code scanning 服务端状态

三个互相独立的只读端点结果一致：

| 请求 | HTTP | GitHub 返回 message |
|---|---:|---|
| `GET /repos/xy7365527-lang/NewChanlun/code-scanning/alerts?per_page=1` | 403 | `Code scanning is not enabled for this repository. Please enable code scanning in the repository settings.` |
| `GET /repos/xy7365527-lang/NewChanlun/code-scanning/default-setup` | 403 | 同上 |
| `GET /repos/xy7365527-lang/NewChanlun/code-scanning/codeql/databases` | 403 | 同上 |

这些端点与错误语义见 GitHub 官方 [REST API endpoints for code scanning](https://docs.github.com/en/rest/code-scanning/code-scanning)。GitHub 的专门排错页也规定：private/internal repository 未启用 GitHub Code Security（或被 policy 阻止）时，Code scanning/SARIF upload 会报此类 403；public repo 则默认可用。【GitHub 官方：[Code Security must be enabled](https://docs.github.com/en/code-security/reference/code-scanning/troubleshoot-analysis-errors/advanced-security-must-be-enabled)、[SARIF upload fails when disabled](https://docs.github.com/en/code-security/reference/code-scanning/sarif-files/troubleshoot-sarif-uploads/ghas-required)】

### 1.3 两个 run 的精确失败面

**CodeQL run 32253705317**：

- `Analyze (actions)`、`Analyze (javascript-typescript)`、`Analyze (python)` 三个 job 都在 `Initialize CodeQL` 前后看到同一服务端警告；但仍完成数据库/查询执行并各自 `Exported results to SARIF`，最后在 `Uploading code scanning results` 得到 403，`Perform CodeQL Analysis` 才失败。【[run](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32253705317)】
- 日志显示 action 默认输入 `upload: always`；本仓 `.github/workflows/codeql.yml` 的 analyze step 没有覆盖它。GitHub 自有 action 清单确认 `always` 是默认值，另支持 `failure-only` 和 `never`。【[github/codeql-action `analyze/action.yml`](https://github.com/github/codeql-action/blob/v4/analyze/action.yml)】

**DevSkim run 32253705319**：

- `Run DevSkim scanner` step=`success`；`Upload DevSkim scan results to GitHub Security tab` step=`failure`。上传 action 已成功找到并验证 `devskim-results.sarif`，随后在 `Uploading results` 得到同一 403。【[run](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32253705319)】
- 这证明 DevSkim 的直接故障域是 GitHub Code scanning 的 SARIF ingestion，不是 scanner 本身。GitHub 官方也明确：private/internal repo 只有启用 Code Security 后才可把第三方 SARIF 显示成 GitHub Code scanning 结果。【[Uploading SARIF](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/integrate-with-existing-tools/upload-sarif-file)】

**据此根因链**：

```text
personal account (Pro) owns a private repository
  → 不是 Team/Enterprise organization-owned repository
  → 不能购买/启用本仓的 GitHub Code Security
  → Code scanning API 固定 403
  → CodeQL 与 DevSkim 都能生成 SARIF，但 GitHub ingestion 拒绝
```

## 2. 当前账号/计划能不能启用

### 2.1 官方资格门

GitHub 的 Code scanning availability 定义只有两类：

1. GitHub.com public repository；
2. 启用了 GitHub Code Security 的 organization-owned repository，组织使用 Team / Enterprise Cloud（或 GHES）。

来源：[Code scanning overview](https://docs.github.com/en/code-security/concepts/code-scanning/code-scanning)。

GitHub 对本题还有更直接的 Free/Pro 排错结论：**Free 或 Pro 只能在公开仓使用 Code scanning；private/internal 必须升级为带 Code Security 的 Team 或 Enterprise，并为 repo 启用 Code Security。** 来源：[Cannot enable CodeQL in a private repository](https://docs.github.com/en/code-security/reference/code-scanning/troubleshoot-analysis-errors/private-repository-enablement)。

GitHub 的计划页进一步区分：Pro 属个人账号；Team 属组织账号，并提供“购买 GitHub Code Security / Secret Protection”的选项。来源：[GitHub's plans](https://docs.github.com/en/get-started/learning-about-github/githubs-plans)。

**因此本仓当前答案是“不能”。** 这不是“账号有 admin 但没找到开关”，而是 owner/account/plan 组合不在产品资格域内。要走原生 Security tab，最小拓扑变化是：

1. 建立 organization；
2. organization 使用 Team 或 Enterprise；
3. 把 repository 转移给 organization；
4. organization/enterprise owner 为 Code Security 建立付费授权（metered 或 volume/subscription）；
5. 在该 repository 上启用 Code Security；
6. 再跑现有 advanced setup workflow。

步骤 2–5 及购买权限见 GitHub 官方 [About GitHub Advanced Security](https://docs.github.com/en/get-started/learning-about-github/about-github-advanced-security) 与 [Buying Advanced Security](https://docs.github.com/en/billing/how-tos/products/buy-advanced-security)。本报告没有执行这些动作。

### 2.2 CodeQL 与第三方 SARIF 的门并不相同

- GitHub Code Security 包含 Code scanning、CodeQL CLI 等；官方 availability 表对“private repository without GitHub Code Security”的 Code scanning 与 CodeQL CLI 都标为 **No**。【[About GitHub Advanced Security](https://docs.github.com/en/get-started/learning-about-github/about-github-advanced-security)】
- CodeQL Terms 的 `License Restrictions` 还逐字禁止：在未由付费 Advanced Security 客户许可解除限制时，用 CodeQL 做未明确授权的 automated analysis/CI/CD，或用于非 Open Source Codebase（官方括号举例即 “code in a private repo in GitHub”）。【[GitHub CodeQL Terms and Conditions](https://github.com/github/codeql-cli-binaries/blob/main/LICENSE.md)】
- 对第三方工具，GitHub 限制的是把 SARIF 上传并显示为 GitHub Code scanning 结果；官方明确 private/internal repository 要先启用 Code Security。【[Uploading SARIF](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/integrate-with-existing-tools/upload-sarif-file)】

所以：

- **CodeQL 的 artifact-only 只是技术上能产文件，不等于获得了在个人 private repo 使用 CodeQL 的产品许可。** GitHub action 的 `upload: never` 输入不能覆盖官方授权门；作为长期退场方案应判不合规。
- **DevSkim 的 artifact-only 不使用 GitHub Code scanning ingestion。** 本仓 run 已实证 scanner 与 SARIF 生成成功；改存普通 Actions artifact 是 GitHub Actions 的通用存档能力，不冒充 GitHub Code Security。

## 3. `gh api` 能否非交互启用，以及权限/计费边界

### 3.1 API 能力：有，但本仓当前不可用

官方 `Update a repository` endpoint 的 request body 支持：

```json
{
  "security_and_analysis": {
    "code_security": { "status": "enabled" }
  }
}
```

来源：[REST — Update a repository](https://docs.github.com/en/rest/repos/repos#update-a-repository)。该文档同时规定：

- 调用者须有 repository admin，或为 owning organization 的 owner/security manager；
- fine-grained token / GitHub App 需要 repository `Administration: write`；
- `GET /repos/{owner}/{repo}` 可检查支持且可见的 `security_and_analysis` 状态。

满足资格后的非交互形态可以是下列命令，但**本轮未执行**：

```bash
# 写操作示意，未执行；ORG 必须先有 Team/Enterprise + GitHub Code Security
printf '%s' '{"security_and_analysis":{"code_security":{"status":"enabled"}}}' \
  | gh api --method PATCH \
      -H 'X-GitHub-Api-Version: 2022-11-28' \
      repos/ORG/NewChanlun --input -
```

当前 viewer 虽是 `ADMIN`，当前 OAuth token 也有 classic `repo` scope，但 repo 仍是个人所有且产品不合资格。**API 是配置通道，不是购买/升级/绕过计费的通道。** 本报告不臆测当前写请求会返回 403 还是 422，因为按边界没有发送它；只判它不能合法成功启用当前拓扑。

组织还可用 code security configurations 把配置应用到多个 repositories；这属于 organization/enterprise 级批量动作，不是本题个人仓的替代入口。【GitHub 官方：[REST API endpoints for code security configurations](https://docs.github.com/en/rest/code-security/configurations)】

### 3.2 权限分三层，不要混为一个“admin”

| 层 | 所需权限/资格 | 本仓当前状态 |
|---|---|---|
| 买产品/建立组织授权 | organization 或 enterprise owner；组织先在 Team/Enterprise【[Buying Advanced Security](https://docs.github.com/en/billing/how-tos/products/buy-advanced-security)】 | 不满足：owner 是个人 User，organization 数 0 |
| 为某 repo 改 `code_security` | repo admin，或 organization owner/security manager；API token `Administration: write`【[Update a repository](https://docs.github.com/en/rest/repos/repos#update-a-repository)】 | 人有 repo ADMIN，但没有产品资格/组织承载面 |
| workflow 上传 SARIF | workflow `GITHUB_TOKEN` 需 `security-events: write`，private repo 还需 `actions: read`、`contents: read`【[Uploading SARIF](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/integrate-with-existing-tools/upload-sarif-file)】 | 已满足；两个 run 的 effective permissions 已实证 |

这解释了为什么“再加一次 `security-events: write`”不会修复：第三层已通过，失败在第一层产品资格/第二层未启用状态。

### 3.3 计费与组织级后果

- GitHub 当前公开标价：Code Security **$30 USD / active committer / month**；最终价格/合同以购买页为准。【[GitHub Security pricing](https://github.com/security/advanced-security)】
- License 用量按启用了 Code Security 的 repositories 中 **unique active committers** 计算；committer 在最近 90 天有 commit 被 push 即为 active；同一人在多个已启用 repo 不重复计多个 license。【[Advanced Security billing](https://docs.github.com/en/billing/concepts/product-billing/github-advanced-security)】
- metered billing 没有预设 license 上限，启用后按当月 active committers 计；volume/subscription 要先买 licenses，容量不足时不能给新 repo 启用，但已有 repo 不会因此自动失效。【同上】
- UI 对 metered enablement 会展示 estimated billing changes 并要求确认；`gh api` 是非交互写通道，不提供这层页面确认。因此自动化 PATCH 应被视为**可能直接扩大付费用量**的审批动作，不能当普通 repo toggle。【[Buying Advanced Security](https://docs.github.com/en/billing/how-tos/products/buy-advanced-security)】
- repository 级 PATCH 只针对一个 repo；organization code security configuration 则可覆盖多仓，可能扩大 active-committer 计费面和策略面。【[Code security configurations REST API](https://docs.github.com/en/rest/code-security/configurations)】
- Code scanning 使用 GitHub Actions；private repo runner minutes 与 artifact storage 另按账号计划计量，超 included quota 会计费。当前 Pro 官方配额为每月 3,000 minutes、1 GB artifact storage；artifact 与 Packages 共用 storage pool。【[GitHub Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions)】

## 4. 不可用时的三类收敛方案

| 方案 | CodeQL | DevSkim | Security tab | fail-loud 能力 | 永久红噪声 | 本报告裁决 |
|---|---|---|---|---|---|---|
| A. 原样保留上传 | 分析后上传固定 403 | 扫描后上传固定 403 | 无 | 只对“授权缺席”响，不对真实 findings 给可用治理 | **必然有** | ❌ 拒绝 |
| B. artifact-only | action 技术支持 `upload: never`，但个人 private repo 无 CodeQL 使用资格 | 已实证可生成 SARIF，可用 `actions/upload-artifact` 存档 | 无 | 需另写 findings policy gate；artifact 本身不判红绿 | 无结构性红 | CodeQL ❌；DevSkim ✅ |
| C. conditional upload | 跳过上传仍在运行无资格的 CodeQL，没解决合规门 | 可先探测 API；已知 403 时存 artifact，200 时上传 | 仅启用后有 | 若“任何探针失败都 skip”会静默；必须区分 expected 403 与 unexpected error | 可无 | CodeQL ❌；DevSkim ⚠️ 只作过渡 |
| D. 停用 workflow | 停用直到获得合资格组织/授权 | 可停，但丢掉现有可用扫描 | 无 | 停用项无检查；须用 issue/配置状态显式登记缺口 | 无 | CodeQL ✅；DevSkim 仅在无维护能力时 |
| E. 迁入合资格组织并启用 | 原生 CodeQL + SARIF ingestion | 原生第三方 SARIF ingestion | 有 | GitHub alerts/checks 可作为 gate | 无结构性红 | ✅ 长期目标，需人审计费/拓扑 |

### 4.1 artifact 的诚实能力边界

GitHub 官方把 workflow artifact 定义为“在 workflow 完成后存储 build/test output，用于调试、覆盖率等”；可以设置每件 artifact 的 `retention-days`，且不得超过 repo/org/enterprise 上限。【[Store and share data](https://docs.github.com/en/actions/tutorials/store-and-share-data)】

对 private repo，artifact 不是公开下载：登录且对 repo 有 read access 的人可下载；默认 logs/artifacts 保留 90 天，可按仓设置调整。【[Downloading workflow artifacts](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts)】

但 artifact **不等于** GitHub Code scanning：没有 Security tab 的 alert lifecycle、branch/PR annotations、baseline/dedup/closure 等。GitHub 官方把这些能力归于 SARIF ingestion/Code scanning；本报告不会把“文件存下来了”写成“code scanning 仍可用”。【[Uploading SARIF](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/integrate-with-existing-tools/upload-sarif-file)、[Code scanning](https://docs.github.com/en/code-security/concepts/code-scanning/code-scanning)】

artifact 还占 Actions storage；当前 Pro included 1 GB，超额按 Actions storage 计费。设置短且明确的 retention、只存必要 SARIF/summary，才能避免把降级方案变成另一个长期账单源。【[GitHub Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions)】

### 4.2 conditional upload 的正确与错误形态

**错误形态**：对 `upload-sarif` 加 `continue-on-error: true`，或探针任何非 200 都直接 skip。它会把权限回退、API 故障、token 配错与“已知无产品”混成绿色，违背 fail-loud。

**可接受的 DevSkim 过渡形态**（这里只定语义，不改 workflow）：

1. entitlement probe 只把精确的 403 + `Code scanning is not enabled...` 归为 `expected-unavailable`；
2. 200 归为 `available`，走 `upload-sarif`；
3. 401/404/429/5xx、响应无法解析、scanner 失败均为红；
4. `expected-unavailable` 时仍 `if: always()` 上传普通 artifact，并运行同一份本地 findings gate；
5. summary 明写 `Security tab upload skipped: entitlement unavailable`，不能静默绿。

该设计基于本仓实测的精确 403 和 GitHub 官方 endpoint 语义；它不改变 CodeQL 的许可结论。【[Code scanning REST API](https://docs.github.com/en/rest/code-scanning/code-scanning)】

## 5. 推荐决策：让 `main` fail-loud，但只对可行动的事响

### 5.1 当前（个人 Pro + private）

1. **CodeQL：停止触发并在 #1106/后续实施票登记“因产品资格停用”。** 不做 artifact-only CodeQL，不把 403 设 `continue-on-error`。恢复条件必须写死为：repo 已迁入 Team/Enterprise organization、Code Security 已购买并对 repo 启用、三个只读 endpoints 不再返回 entitlement 403。
2. **DevSkim：保留 scanner，退到 artifact mode。** 普通 artifact 在 success/failure 都留存；另设明确的 SARIF findings policy gate。只有 scanner/解析故障、artifact 缺失、或 findings 超过仓库裁定阈值才红。
3. **若当前没有人能定义/维护 DevSkim findings policy gate，则宁可也停用 DevSkim并保持 #1106/实施票开放，不要造一个“扫描但永远绿”的假闸。**
4. **不要用 conditional upload 掩盖当前固定不可用状态。** 它只适合未来迁移期或跨 public/private 的复用 workflow；当前单仓已知永久 403 时，直接选择 artifact mode 更简单、更诚实。

### 5.2 将来获授权后

1. organization/enterprise owner 审核 Team/Enterprise、Code Security license 模式、active committer 估算、budget/policy；
2. 转移 repo，并经 UI（有 billing estimate）或审批后的 repo 级 PATCH 启用；
3. 用本报告三个 GET probe 验证不再是 entitlement 403；
4. 再恢复 CodeQL 和 DevSkim 的 GitHub-native SARIF upload；
5. 首次真实 run 验证：分析、上传、Security tab processing、alert/check 全链，而不只看 workflow 文件存在。

### 5.3 一句话的红绿口径

> **已知且暂不可改变的产品缺席应是显式登记的 neutral/disabled 状态；扫描器故障、状态漂移、无法判定、以及超过已裁阈值的真实 findings 才应 fail-loud。**

这能消除“每次都报同一个不可处理 403”的永久红，同时不把上传丢失或扫描失败漂成绿色。

## 6. 本轮只读探针附录

以下命令均为 GET/GraphQL query 或读取 Actions 日志；没有写调用。输出中的 token 未打印。

```bash
# 基线与远端 main 一致性
git rev-parse main
git ls-remote origin refs/heads/main

# 仓库拓扑与 viewer 权限
gh api graphql -f query='query {
  repository(owner:"xy7365527-lang", name:"NewChanlun") {
    nameWithOwner isPrivate visibility owner { __typename login } viewerPermission
  }
  viewer { login organizations(first:100) { totalCount nodes { login } } }
}'

# 账号公开计划 badge（只读公开页）
curl -fsSL https://github.com/xy7365527-lang \
  | grep -o 'title="Label: Pro"' | head -1

# Actions 与 Code scanning 状态
gh api repos/xy7365527-lang/NewChanlun/actions/permissions
gh api repos/xy7365527-lang/NewChanlun/actions/permissions/workflow
gh api -i 'repos/xy7365527-lang/NewChanlun/code-scanning/alerts?per_page=1'
gh api -i repos/xy7365527-lang/NewChanlun/code-scanning/default-setup
gh api -i repos/xy7365527-lang/NewChanlun/code-scanning/codeql/databases

# 真实 run/step/token-permission/失败日志
gh run view 32253705317 --repo xy7365527-lang/NewChanlun --json jobs
gh run view 32253705319 --repo xy7365527-lang/NewChanlun --json jobs
gh run view 32253705317 --repo xy7365527-lang/NewChanlun --log
gh run view 32253705319 --repo xy7365527-lang/NewChanlun --log
```

## 7. 一手来源索引

### GitHub 官方产品、资格与计费

- [Cannot enable CodeQL in a private repository](https://docs.github.com/en/code-security/reference/code-scanning/troubleshoot-analysis-errors/private-repository-enablement)
- [About GitHub Advanced Security](https://docs.github.com/en/get-started/learning-about-github/about-github-advanced-security)
- [GitHub's plans](https://docs.github.com/en/get-started/learning-about-github/githubs-plans)
- [GitHub Advanced Security license billing](https://docs.github.com/en/billing/concepts/product-billing/github-advanced-security)
- [Buying Advanced Security](https://docs.github.com/en/billing/how-tos/products/buy-advanced-security)
- [GitHub Security pricing](https://github.com/security/advanced-security)
- [GitHub Actions billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions)

### GitHub 官方 Code scanning / SARIF / REST

- [Code scanning](https://docs.github.com/en/code-security/concepts/code-scanning/code-scanning)
- [Code Security must be enabled](https://docs.github.com/en/code-security/reference/code-scanning/troubleshoot-analysis-errors/advanced-security-must-be-enabled)
- [SARIF upload fails because Code Security is disabled](https://docs.github.com/en/code-security/reference/code-scanning/sarif-files/troubleshoot-sarif-uploads/ghas-required)
- [Uploading a SARIF file to GitHub](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/integrate-with-existing-tools/upload-sarif-file)
- [REST API endpoints for code scanning](https://docs.github.com/en/rest/code-scanning/code-scanning)
- [REST — Update a repository](https://docs.github.com/en/rest/repos/repos#update-a-repository)
- [REST — Code security configurations](https://docs.github.com/en/rest/code-security/configurations)
- [GitHub CodeQL Terms and Conditions](https://github.com/github/codeql-cli-binaries/blob/main/LICENSE.md)
- [GitHub CodeQL action `analyze` inputs](https://github.com/github/codeql-action/blob/v4/analyze/action.yml)

### GitHub 官方 Actions artifact

- [Store and share data with workflow artifacts](https://docs.github.com/en/actions/tutorials/store-and-share-data)
- [Downloading workflow artifacts](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/download-workflow-artifacts)

### 本仓 GitHub 现场

- [CodeQL run 32253705317](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32253705317)
- [DevSkim run 32253705319](https://github.com/xy7365527-lang/NewChanlun/actions/runs/32253705319)
- [当前账号公开页（Pro badge）](https://github.com/xy7365527-lang)
- [issue #1106](https://github.com/xy7365527-lang/NewChanlun/issues/1106)
