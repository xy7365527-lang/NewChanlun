# LeveledMove.sub_moves Rc 化（OOM 根因2）— Task #104

认识论等级：**L0**（纯结构重构，无经验数据；bit-exact 由 L1 测试护栏验证）。

## 结论

`LeveledMove.sub_moves: Vec<LeveledMove>` → `Rc<Vec<LeveledMove>>`。compose 逐级深拷贝整棵子
move 树消除——更高级父节点 clone 子树时只 clone Rc 引用计数（O(1)），子树物理单份。
cargo test --lib **1302 passed; 0 failed**（= baseline，无降），bit-exact 不破。

## 改动清单（最小 diff）

| 文件 | 改动 | 性质 |
|------|------|------|
| recursive_tower.rs | `sub_moves` 字段 `Vec`→`Rc<Vec<..>>`；`from_unit`/`compose` 构造包 Rc；`descend_leveled` 返回 `Rc<Vec<..>>`（`Rc::clone` 替深拷贝） | 核心 |
| mod.rs:1019/1021 | `descend_leveled` 返回 Rc，传 `&[..]` 参数处 `&subs`→`&subs[..]` | 调用适配 |
| voice_eat.rs / coverage.rs / interp.rs | `for sub in &x.sub_moves`→`x.sub_moves.iter()`（`&Rc<Vec>` 无 IntoIterator） | deref 适配 |
| voice_eat.rs 测试 | 字段字面量构造包 `Rc::new` | 测试适配 |

`.len()`/`.is_empty()`/`[i]` 等方法/Index 调用 auto-deref 透明，零改动。

## 不变量保持（recursive_tower.rs:77-85）

- `sub_moves[i].rmove == rmove.subs[i]`：构造逻辑未动，子序列内容不变。
- `descend_leveled` 取回 = `Rc::clone`，deref 后调用方得同一 slice，与旧 `.clone()` 内容等价。
- `index_of_in` 映射原始 K 序：消费 `&subs[..]`，逻辑不变。
- L0 线段 `sub_moves` 空：`Rc::new(Vec::new())`，`.is_empty()` 仍真。
- `PartialEq`/`Eq`：`Rc<Vec<T>>` 按 deref 内容比较，测试 `assert_eq!(ru, full_u)` 整树比较仍成立。

## 线程安全

`LeveledMove` 不跨线程（全 crate 无 `thread::spawn`/`rayon`/`par_iter` 消费它），`Rc` 足够，
不需 `Arc`。

## 边界条件（结论翻转点）

1. 若日后 classifier 塔被引入多线程消费（rayon 并行回测等）→ `Rc` 非 Send/Sync，编译失败，
   须升 `Arc`（O(1) 改动，仅换类型别名）。
2. **此优化只消除深拷贝的一半**：`LeveledMove` 持双份子树——`sub_moves`（已 Rc）+ `rmove`
   的 `RMove::Compose.subs: Vec<RMove>`（未动，在 descend.rs，Lean μF bit-exact 镜像）。
   `rmove.subs` 同样随级别深度递归膨胀。若 OOM 仍未解，根因2 的另一半在 `RMove::Compose.subs`
   ——但该结构在 descend.rs（本任务边界外，且是 Lean 镜像，改动须异质审计）。

## 影响声明

改动 classifier 核心对象 `LeveledMove`（主引擎依赖）。bit-exact 由 1302 全量测试护栏验证
未破。未触及 econ_positive.rs（#103 owner）。**未触及 descend.rs**（`rmove.subs` 深拷贝半边
未动，见边界条件 2）。

## 内存改善

L0 重构，未实测 RSS（300K bar 峰值对比需运行回测，本任务真封以 bit-exact 全量通过为准）。
理论：tower 深度 d 级时，旧版每个 L_k 父节点深拷贝其完整 L_{k-1..0} 子树（拷贝量 ∝ 子树节点
数）；Rc 后子树单份共享，clone 仅引用计数。膨胀从 O(节点数 × 平均深度) 降至 O(节点数)。

## 谱系引用

不涉及概念分离领域（纯存储表示重构，不改走势/中枢/买卖点任何定义）。
codex Q4 ElementId 确定性身份不受影响（Rc 不改 `id` 字段）。
