# code-verify-prop4-20260623

**验证时间**: 2026-06-23  
**验证 commit**: 79257d6e46 (`feat(recursive_t): 任务22 读法乙双向consume平空+严格逐级区间套(开放轴C)实装+harness`)  
**验证者**: code-verifier（结构工位）

---

## 结论：测试通过，存在1条新增死代码（no-patch 违规）

### 测试结果（简化版结果包）

**结论**  
- 全量测试：**536 passed, 0 failed, 24 ignored**（`cargo test` 完整输出）
- `recursive_t` 专属：**102 passed, 0 failed, 16 ignored**
- bit-exact OFF 守住：`rec_flat_bit_exact_含走势完成清仓路径` → **ok**
- `prop4_consume_l3` / `prop4_nest_l3`：**ignored**（需 `analysis/data_cache/*.json` 数据文件，非失败）

**边界条件**  
- L3 harness 测试（prop4_consume_l3 / prop4_nest_l3）被 ignored，全量验证未运行
- bit-exact 验证仅覆盖合成序列路径（L1 级，非 BTC 真实数据）
- `rec_flat_btc_bit_exact` 同样 ignored（需 btc_1m_full.json）

**影响声明**  
- 本验证不改动任何代码
- 测试通过结论仅对已执行的 536 个测试有效

---

## 死代码发现（no-patch 违规，需修复）

### 新增死代码（本 commit 引入）

| 位置 | 类型 | 说明 |
|------|------|------|
| `rec_engine.rs:443` | `TRoot.reading_b_diverge: bool` | 字段从 `EngineConfig` 复制到 `TRoot` 构造（line 535），但从未在任何逻辑中被 **read**。 |

**确认方法**：  
- `git show b4f20284d8:rust/src/recursive_t/rec_engine.rs | grep "reading_b"` → 空（前一 commit 无此字段）
- 当前文件中 `reading_b_diverge` 所有引用：lines 77/92/103/110/114/117（EngineConfig定义）+ 443（TRoot字段定义）+ 535（构造赋值）
- **零读取**：没有任何行读取 `self.reading_b_diverge`
- `cargo build` 编译警告：`field reading_b_diverge is never read`

**no-patch 规则引用**：  
- 090号：严格性语法规则——非严格产出不合法  
- no-patch-mentality.md §"禁止的模式" 第4条：TODO 遗留——修完后留下"后续优化"的尾巴

### 预存在死代码（非本 commit 引入，供参考）

| 位置 | 类型 | 引入 commit |
|------|------|-----------|
| `rec_stream.rs:95` | `RecStream.cfg: EngineConfig` | b4f20284d8（上一 commit） |

此条预存在，不属于本次验证范围的修复要求，但可一并清理。

---

## bit-exact OFF 状态

`rec_flat_bit_exact_含走势完成清仓路径` 测试通过，确认：

- `EngineConfig::off()` 路径（所有 nest_* flags = false）产出与 flat 逐位一致
- flag 门控有效：nest_consume / nest_strict 路径在 OFF 模式下不激活

---

## 修复建议（供 Lead 派工位）

需在 `rec_engine.rs` 中删除 `TRoot.reading_b_diverge` 字段及其构造赋值（line 535），直到该字段在翻转逻辑中真正被消费。

**判断**：这是 no-patch 规定的"删除并用正确逻辑替换"——当前正确形式是「先删除，待有具体使用逻辑时再加回来」。

