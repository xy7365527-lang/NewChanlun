# ADR 0046：s-admission 锚是 S 资源准入合同——首签主体、签发与撤销程序、无锚库接管

> 正本：本文件是 S 结构服务持久准入锚 `s-admission` 的名分、首份锚数值来源与签发主体、签发/撤销前置与拒绝条件、过期含义的正本。
> 最后裁定：#1567（2026-10-21，用户批复）
> 受影响代码清单：见「受影响代码与票」节（实施票 #1657）

**日期**：2026-10-21
**裁定人**：用户（2026-10-21 批复 [#1567 评论 6310847303](https://github.com/xy7365527-lang/NewChanlun/issues/1567#issuecomment-6310847303)，原话「不重要你自己定吧」，委托主控按推荐默认值逐项确认）；代理起草，非作者独审
**状态**：已采纳（待 PR 合入）。签署单、考古与独审：`/Volumes/AgentStorage/decisions/1567/{draft.md,archaeology.md,review.attempt3.json}`；落地独审 `.chanlun/review-results/pbatch-land-20261021/`。

## 背景

R4（#1527 整合分支 `codex/1527-owner-integration-20261019`，R4 合入点 `0ade553a`，尚未进 main）让 S 控制器启动时只消费一份已发布的持久锚：`s-admission/` 下 `binding.json`（库身份六字段 db、path、sha256、file_identity、binding_identity、directory_identity，`rust/src/bin/s_session_v3/admission_anchor.rs:166-199`）+ `limits.json`（`rust/src/s_content/job.rs:95-106` 的 `JobLimits` 八项，「没有 `Default`」）；消费端「不创建、修复、重签或删除对象」（`admission_anchor.rs:1-6`）。全仓只有 `#[cfg(test)]` 下的 `test_prepare`（`:1303-1306`）能造锚，生产 S 服务因此在任何库上都起不来，#1527 拿不到正式成功收据。

## 裁定

### 一、锚的名分与沿用条款

1. 锚是 S 结构服务的**资源准入合同**（库身份 + 八项计算/存储/时间上限），只准 S 结构服务在该库上启动，**不授予任何交易发送权**。生产与实验之分由 TB-09 与 P2 首发授权把关（[ADR 0028](0028-first-capability-verification-profile.md):46/:48），锚不加场所、证据类或墙钟有效期字段——加字段要改 R4 的六字段 exact 检查。
2. 不存在运行时自动接管（[#1527 根裁定 6245891166](https://github.com/xy7365527-lang/NewChanlun/issues/1527#issuecomment-6245891166)「不得……无锚自动接管」；`CONTEXT.md:169`「旧库……不迁移、不兜底」；AGENTS.md 收敛通则）。
3. 换库锚由 R4 现有身份检查拒绝（`admission_anchor.rs:196-198`、`:370-388`）。
4. 不事后追认：签锚只对签发之后的启动生效（`CONTEXT.md:444`「AsKnown 不回填」同型）。
5. `test_prepare` 只在测试构建可达，生产签发与它互不可达（#1527 已交，评论 6267771837）。
6. 八项数值不由用户报；`max_store_bytes` 引 [ADR 0037](0037-s-content-bounded-layout.md):24 既裁值（bounded-layout/1 每库 64 GiB），本 ADR 不另裁、不改值；测试夹具值不得进生产合同。

### 二、首份锚数值来源与签发主体

首份锚数值来自一份具名、版本化的「S 资源准入合同」：八项限额 + 适用布局/库，由实施票 #1657 基于实测提出，主控（根）按 #1323 Notes N-1 审定并经非作者独审后落 `docs/adr/`，同时登记合同规范 JSON 的 sha256 版本清单。先例：ADR 0037:4「代理依用户 2026-10-13 授权裁定……根逐条复核」裁资源上限；[#1527 根裁定 6246445786](https://github.com/xy7365527-lang/NewChanlun/issues/1527#issuecomment-6246445786)「生产启旧域须在具名新准入合同显式绑定该计量域及统一上限，未具名授权仍拒绝」。

签发动作由操作员以正式子命令执行（[ADR 0027](0027-classification-chong-operating-program-ownership.md):58「控制动作必须经正式 command……进入生产程序」）。命令不按人的身份把关，把关点是两条机器可判条件：

- (i) 进程 uid 等于服务 uid（R4 身份检查同口径）；
- (ii) 输入的合同文件规范 JSON 的 sha256 出现在已审定合同 ADR 的版本清单里，且为当前生效版本（被取代版本拒绝）。

操作员可以是用户，也可以是主控派发的代理。实施负责人、工蜂、CI 不得自拟数值签锚（主控 [#1527 评论 6267327842](https://github.com/xy7365527-lang/NewChanlun/issues/1527#issuecomment-6267327842)）。

### 三、签发前置与拒绝条件

签发命令拒绝：库已有 published 或 pending 锚（不覆盖、不重签；换锚须先显式撤销，撤销留档）；库正被 S 写者占用；未给出已审定合同版本；库文件 uid/mode 不符 R4 要求。签发后 R4 照旧判 `Identity` / `Changed` / `AnchorConflict` / `AnchorLost`（`admission_anchor.rs:370-388`；`s_session/recovery_admission.py:198-202`）。镜像 R4 检查是 `[新缠论:推论]`。

### 四、「过期锚」的含义

锚的 `limits.json` 与当前生效合同版本登记的规范 JSON sha256 不一致即过期（合同被新版本取代后旧锚即属此类），启动器在调用 R4 之前比对并拒绝启动。不另设墙钟有效期（资源上限无时间属性，ADR 0037:24 无期限），不改 R4 的六字段/八项 exact 检查。已审定 sha256 清单的载体不得是同 uid 进程可无身份校验改写的普通文件，方案由 #1657 提出、按 N-1 审。取「与生效合同不一致即过期」是 `[新缠论:选择]`。

### 五、分级与后续变更

用户已确认第二节的授权层级。此后合同版本更迭、再签发、撤销及其负控属 T 类（`.sandcastle/docs/graph-engineering.md:23`「恢复语义」），按 #1323 Notes N-1 自动裁决。

### 六、票间切分与负控

实施票 [#1657](https://github.com/xy7365527-lang/NewChanlun/issues/1657) 交付：资源合同首个版本（交付前置）、签发子命令（候选名 `s_structure_session admission-prepare`，最终名归 #1657）、撤销子命令、启动前「锚 limits 与生效合同 sha256 一致」校验、release 无 `test_prepare` 的 CI 断言沿用，以及三组负控（判据二值）：

1. 伪造锚被拒：limits 字节改动致 sha 不符、binding 身份伪造、非服务 uid 的文件；
2. 过期/换库锚被拒：锚 limits 与生效合同 sha256 不一致；库复制到新 inode/设备后用原锚启动；
3. 无授权签锚被拒：未给合同版本、合同 sha256 不在已审定清单、进程 uid 不是服务 uid、库已有锚或 pending。

#1527 只交 R4 消费端、独审、二进制身份与失败收据，不在票内签锚；其正式成功收据被 #1657 阻塞。签发/撤销子命令属生产入口，须进入 [ADR 0039](0039-production-entry-inventory-and-unique-launcher.md)（#1530）的入口清单。

## 被否方案

- **(a) #1527 内加操作员命令、由实施负责人自拟数值首签**：主控 6267327842「不由实施负责人在票内定」；根 6246445786「未具名授权仍拒绝」。
- **(c) 运行时自动接管（无锚即用内置限额自建锚）**：根 6245891166 明禁；违反收敛通则与 `CONTEXT.md:169`。
- **(d) 事后追认**：同 `CONTEXT.md:444` 不回填。
- **(e) 把锚归入用户签署的资金政策族、绑定场所 profile 与 evidence_class、带墙钟有效期**：锚字段全集没有这些内容；交易权限另由 ADR 0028:46/:48 把关。

## 受影响代码与票

- `rust/src/bin/s_session_v3/admission_anchor.rs`（R4，`0ade553a`）：消费端不改；签发/撤销在 #1657 新增。
- `s_session/recovery_admission.py`：不改。
- [#1657](https://github.com/xy7365527-lang/NewChanlun/issues/1657)：blocked-by #1567，blocking #1527。
- [#1527](https://github.com/xy7365527-lang/NewChanlun/issues/1527)：正式成功收据被 #1567 与 #1657 阻塞。
- #1530 / ADR 0039：#1657 的子命令入生产入口清单。

## 溯源标签

全部 `[新缠论:选择]` 或 `[新缠论:推论]`（第三、四节已逐条标注）；缠师原文对 S 服务资源上限无论述。
