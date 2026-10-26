# #1669 正本覆盖面补正记录（2026-10-25）

## 缺口与证据

最终独立复核 `REVIEW-BEFORE.md` 判定 `3af30f1157` FAIL/BLOCK：票面要求的 `.chanlun/definitions/sanjieduan.md` 实际声明没有进入候选。当前分支四个实现/返修提交均只改 `rust/src/economic_session/tb03c_book_tests.rs`，后续提交只加入评审记录；`formal/` 与 `.chanlun/definitions/` 在 `origin/main...3af30f1157` 中均为零改动。原三轮评审却引用了这些文件，故不能作为该候选最终验收证据。这里不是“有内容被 rebase 丢失后可恢复”：当前分支历史中不存在那些声明，必须实际补交。

## 本次补正

- 在 `sanjieduan.md` §2 原“证明与对拍义务”段后新增 **实际覆盖面登记（#1669，2026-10-25）**，原义务句及 #1590 订正均保留。
- 声明只覆盖与本义务相关的 Lean 范围，不把 `TotalWealth.lean` 的全部定理缩写成“只有阶段单调性”。
- 明确 `stage_rank_monotone` / `earning_no_regress` 的单步与 `clearCampaign` 排除边界、`TWState` 无 `side` 但 `units` 带方向注记、模型无 `growth_eligible` 对应谓词及资格蕴含定理。
- 明确 `LegalEnterEarningStrict` 是含 `stage = capitalRecovered` 的严格 EnterReady 入口合法性谓词，不是扩量资格谓词。
- 多头与空头资格蕴含及空头不进 `EarningShares` 均由 Rust 同输入性质锁承担；跨方向归 #1378。
- 不修改 `formal/`、生产代码、测试或冻结夹具。PR 正文同步改成当前真实写集；新 head 仍须独立复审。

## 当前验证边界

本次是正本与证据补正，不新增运行行为。已核对 Lean 源 `TotalWealth.lean:97-109,194,212,657-675` 与正本引用一致；未运行本机 cargo。既有 Rust 性质测试、变异记录与 CI 证据不因文档补正被改写为全绿，main/#1621 三项 funding 基线红仍单列。
