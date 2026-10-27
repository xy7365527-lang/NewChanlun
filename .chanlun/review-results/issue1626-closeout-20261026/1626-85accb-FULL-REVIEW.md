# #1626 / PR #1693 非作者独审

- 候选：`85accb9270a52330f193a4dcc93aa93378e1e2b8`（与当前 PR head 一致）
- 基线：`origin/main @ 3ff38accdd93a3fddac6e1a5274b7cb7d192ac0f`
- 范围：Q-4 定义回溯、生产调用与反例。依审查纪律，概念层发现矛盾后停止，不进入其余工程层验收。
- 方法：对照 ADR 0041、`qujiantao.md` §14.2、当前 diff、`positional` 生产链及测试源码；未在本机运行 cargo/rustc。早期报告 `/Volumes/AgentStorage/issue1626-rust-progress-review-20261025/REVIEW.md` 已复核。

## 代码审查报告

### 概念层质询结果：发现矛盾（HIGH）

**H-1：早期报告的 Q-4 预注册绕过在当前 head 仍然存在。**

`qujiantao.md` §14.2 规定：`Ancestor` 只对应 `hold26_anc`，`SelfOrAncestor` 只对应 `fusion_ta`，且只在各自预注册研究臂下合法；其他生产 profile 需要另外的预注册与准入依据。

当前实现没有执行这个 profile—值边界：

1. `rust/src/theta_v0/policy_records.rs:379-398` 对三个 profile 都接受全部三种值，没有按 profile 限定允许值。
2. `rust/src/trading/positional.rs:1135-1154` 把 `policy.value` 直接传给 `run_fusion`，没有补充生产准入检查。
3. `rust/src/trading/positional_fusion.rs:502-524` 的守卫只检查其它轴，无法识别原调用模式究竟是 `fusion_t`、`hold26_anc` 还是 `fusion_ta`。
4. `rust/src/theta_v0/policy_records.rs:801-807` 的测试明确接受 `default=Ancestor`，把越界反例固化为合法行为，而不是拒绝它。

**反例**：把新版本记录的 `default` 改为 `Ancestor`。`fusion_t` 原模式选择 `TrendScopeProfile::Default`，随后取得 `Ancestor` 并进入 `run_fusion`；当其它轴满足现有守卫时，系统便获得正本未预注册的第三套祖先臂。`hold26_anc` 与 `fusion_ta` 也可被分别改成不属于各自预注册身份的值。

因此，当前“版本化记录 + 版本戳”只证明读取了记录，没有证明记录值获准用于该生产 profile。实施报告所列测试不能消除这项语义矛盾。

**必须返修**：在记录解析或生产准入处绑定 profile 与允许值：`default=SelfLayer`、`hold26_anc=Ancestor`、`fusion_ta=SelfOrAncestor`；其它组合 typed reject。补生产入口的三组负控，证明 `fusion_t` 的两种越界值以及 `hold26_anc`/`fusion_ta` 各自越界都在进入 `run_fusion` 前失败，不能只测解析函数或继续接受越界。

### 工程层审查

概念层未通过，按纪律不进入工程层审查。

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | — |
| HIGH | 1 | fail |
| MEDIUM | 未审 | — |
| LOW | 未审 | — |

**结论：FAIL / BLOCK。当前 head 不具备合入或关闭 #1626 的资格。**

## 其它范围与验证状态

- Q-6～Q-8、Q-2、X-38 及 Lean 全链：因概念层阻断，本次不作完成判断，均为 **NOT_VERIFIED**，不是发现它们失败，也不以早期局部结论替代整票复审。
- nc-remote 155 passed / Python 94 / Lean 161 jobs / fixture 字节一致：已阅读实施报告，未把作者所列结果当作本审独立运行证据。
- GitHub CI 仍在执行；即使转绿，也不能消除 H-1。
- 返修后必须以新的真实 head 重新进行整票独审，并重新核对 Q-4 与后续 Lean/对拍范围。
