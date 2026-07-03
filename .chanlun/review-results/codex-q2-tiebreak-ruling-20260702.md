# codex tie-break 终裁：Q2 双终裁冲突——619 适用域裁断

- **工位**：ws-codextie（codex-challenger，goal g-full-strategy-pi p2 硬前置）
- **日期**：2026-07-02（跨 2026-07-03 UTC 边界完成）
- **审计对象**：两次独立 Codex decide 调用的相反结论——`codex-q2-d1-ruling-20260702.md`（裁定一：删 `mutex_interp.rs` 留 `mutex.rs`）vs `codex-q2-conflict-note-20260702.md`（裁定二：删 `mutex.rs` 留 `mutex_interp.rs`）
- **Codex 完整交互**：`.chanlun/review-results/codex-decide-20260703-001041-e04f.md`（自动持久化，含完整 prompt + response）
- **裁定结论**：**选 A——619 号不覆盖 `rust/src/theta_v0/` 下的 Rust 测试辅助模块，维持裁定一（删 `mutex_interp.rs`，留 `mutex.rs`）**

---

## 1. 结论

**619 号的适用域裁断**：619 号（"formal/Origin/ 六层 = 唯一 canonical base"）的约束对象是 `formal/` 目录内的 **Lean 证明文件**，不覆盖 `rust/src/theta_v0/` 下自称"镜像 Lean"的 Rust 测试辅助模块。

依据（引用 619 原文，见 §2）：619 的 `separation.after`、`impact.affected_modules`、`new_output.code_changes` 三处枚举的具体受影响对象**全部是 `formal/` 目录路径**（`formal/Origin/`、`formal/Strict/`、`formal/Tlayers/`、`formal/Foundation/`、`formal/theta_v0`），没有一处提及 `rust/src/` 路径。619 全文语境（含前置 618 号）围绕的是"哪个 Lean 目录是 `formal/` 的权威基座"这一特定问题，不是"任何自称对齐 Lean/Origin 的代码，无论语言、无论目录"。

**关键澄清（本工位新发现，供团队记录）**：`ws-codexp2` 的误判很可能部分源于命名巧合——619 原文提到"theta_v0 → 降为待重锚库"，指的是 `formal/theta_v0`（Lean 参数化雏形子目录）；而本次冲突的 `mutex_interp.rs` 位于 `rust/src/theta_v0/closed_loop/`——**这是同名但完全不同的两棵目录树**（前者是 619 治理对象，后者是生产代码根目录）。这个同名巧合容易造成"619 已经点名 theta_v0，所以覆盖这里"的误读，实际上不成立。

**最终去留**：
- `strategy/mutex.rs`：**保留**，身份限定为 D1 桶级等价 property test 的 shadow oracle（非 formal canonical base，非 Origin port）。
- `closed_loop/mutex_interp.rs`：**删除**，不作为 D1 oracle 复活。
- `mutex.rs` 现有的 `predicates_of()` / `bridge_bucket()` / `shadow_fold_bucket_equivalence` 实装方向continue（工位 #94 已按此方向实装，本裁定确认其方向正确，无需回退）。

**否定路径的驳回理由**（Codex 原文）：
- B（扩张 619 覆盖所有"镜像 Lean"的 Rust 代码）被拒绝：一个 Rust 模块文档注释不能自动把测试脚手架升级为 formal canonical artifact，否则目录边界和治理对象失效。
- C（双删 + 自建 oracle）被拒绝：`mutex.rs` 已经是"明确只测 Rust 生产桶等价、不宣称 formal 权威"的形态，重建是多余劳动。
- D（10类→三桶投影 wrapper）被拒绝：`Deleverage`/`ExecutionRepair`/`PhaseThreeAccreteCore` 等细分类别在投影映射下要么无生产对应要么需要不可达性证明，当前未被证明良定义，是为 D1 加一层解决错误问题的复杂度。

## 2. 定义依据

- 619 号原文 `separation.after[0]`："Origin 六层是 `formal/` 的**唯一 canonical base**——所有后续形式化(完全分类忠实编码 + 全定义策略 + T 层零件)向 Origin 对齐实例化"。
- 619 号原文 `impact.affected_modules`：枚举六项，全部为 `formal/Origin/`、`formal/Strict/`、`formal/Tlayers/`、`formal/Foundation/`、`theta_v0`（Lean 子目录）、`#89/#90/#91` 缺口、`#95` 吸收物——无一涉及 `rust/src/` 路径。
- `mutex_interp.rs` 本身是 `.rs` 文件（Rust），不是 `.lean` 文件，物理上不在 `formal/` 目录树下，不满足 619 `separation.after[1]`（待重锚库对象）定义的"已编译通过的零件"这一 Lean 语境描述。
- 生产路径 `strategy/interp.rs::interpret` 的实际桶级输出（close/open/record 三桶）是 D1 property test 要验证的比较面——`mutex.rs` 8 谓词/三桶粒度与之匹配，`mutex_interp.rs` 9 谓词/10 类粒度更细且无生产对应（裁定一原有论据，本次确认维持）。

## 3. 边界条件（裁决翻转条件）

- 若后续有新结算谱系明确规定"`formal/Origin` 是所有语言实现（含 Rust）、所有测试 oracle、所有镜像模块的唯一语义来源"（即显式扩张 619 的适用域到跨语言/跨目录），本裁决翻转，需重新评估 `mutex_interp.rs` 的 Origin port 地位。
- 若 D1 的验证目标从"Rust `interp::interpret` 桶级等价"改为"Rust 对 `Origin.FullDefinitionStrategy.chooseAction` 的忠实 port 一致性验证"，本裁决翻转（此时 `mutex_interp.rs` 的 9 谓词镜像才是恰当 oracle）。
- 若生产 `interp::interpret` 未来升级为 9/10 类细分动作语义（而不再是三桶粗粒度），本裁决翻转。
- 若 `mutex_interp.rs` 一方提供并证明了总定义、无歧义的"10 类→三桶"投影（含不可达类别的形式化证明），候选路径 D 重新具备可行性，可覆盖本裁决。

## 4. 下游推论

- #94（p2-impl 工位）当前的实装方向（已删 `mutex_interp.rs`、已扩展 `mutex.rs` 含 `predicates_of`/`bridge_bucket`/shadow-fold test）**与本裁决一致，无需回退**，可继续推进收尾（cargo test 护航 + commit）。
- 619 号谱系本身**不需要修订**——本次裁断不是否定 619，而是澄清其适用域边界（619 治理 `formal/` 内 Lean 权威基座问题，未曾也不需要延伸到 `rust/` 生产代码模块去留决策，二者是不同层级的治理对象）。
- 建议 619 号谱系（或新增一条轻量谱系记录/术语澄清）补充一句显式声明："本号约束对象限定于 `formal/` 目录内的 Lean 形式化文件，不延伸约束其他语言/目录中自称'对齐/镜像'某个 Lean 构造的代码"——避免未来类似的同名目录（`theta_v0`）混淆重演。此为非阻塞建议，供 genealogist 工位酌情处理。

## 5. 谱系引用

- `.chanlun/genealogy/settled/619-origin-canonical-base-A-prime-port-not-reprove.md`：本次裁断的直接审计对象（澄清其适用域，非修订其内容）。
- `.chanlun/review-results/codex-q2-d1-ruling-20260702.md`：裁定一（功能适配性论据），本次确认维持。
- `.chanlun/review-results/codex-q2-conflict-note-20260702.md`：裁定二 + 冲突记录（治理合规性论据），本次裁断驳回其"619 覆盖 rust/ 侧代码"的前提，但认可其提出的问题拆分本身有价值（"D1 oracle 功能问题"vs"619 治理合规问题"确实是两个不同问题，只是后者在本案不适用）。
- no-patch-mentality（090号语法规则）：维持"零消费者权威镜像不并存"的删除依据。

## 6. 影响声明

- 本工位纯只读审计 + Codex CLI 调用，未修改任何生产代码。
- 产出：本裁定文件 + Codex 完整交互记录（`.chanlun/review-results/codex-decide-20260703-001041-e04f.md`）。
- 下游消费：#94（p2-impl）确认方向正确，可继续推进而非回退；genealogist 可酌情在 619 上补一句适用域澄清（非阻塞）。

---

```yaml
---stance-declaration---
verdict: settled
review_target: Q2双终裁冲突 tie-break（619适用域）
stances:
  q2_choice: A
  q2_619_scope_covers_rust_dir: rejected
  q2_619_scope_limited_to_formal_lean_files: confirmed
  q2_delete_mutex_interp_keep_mutex_class: confirmed
  q2_naming_collision_theta_v0_formal_vs_rust_flagged: confirmed
  q2_task94_direction_correct_no_rollback: confirmed
escalation_needed: []
concessions:
  - "若团队后续正式扩张 619 覆盖跨语言/跨目录镜像代码，本裁决按边界条件翻转"
  - "建议 619 补一句适用域显式声明防止 theta_v0 同名混淆重演——非阻塞，genealogist 酌情处理"
---end-stance---
```
