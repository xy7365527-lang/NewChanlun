# #1603 / PR #1624 非作者独审记录

审查者：claude CLI `claude-opus-5-5`（经 CLIProxyLocal 127.0.0.1:8318，独立会话、未参与实施；只读，未跑 cargo）。实施者：/root/owner_1603（Codex）。

## 第 1 轮（审 0671ddfe80）

结论：PASS-with-notes

这次审阅只读，没有跑 cargo。下面的判断都来自读 diff、读源文件和用 rg 检索。PR 里报的测试数和变异体结果，我在本地没有复现。

---

**1. 只从版本化记录读值，输出写记录 id 和版本：PASS（附 note）**
- 读值：`policy_records.rs:29,31` 用 `include_str!` 把两份 JSON 编进二进制。`config.rs:109-111`、`:135-137` 的 `Default` 和 `config.rs:483-486` 的 `try_from_active_records` 都只走 `parse_*`，代码里没有 6、12、26、9 这几个字面量。
- 输出：`pipeline.rs:393-396` 在 `ClassifyOutput` 里加了 `termination` 和 `policy`。`:546-553` 的 `TowerPolicyStamp::of(config)` 把两份记录的版本写进输出。
- 我自己用 rg 搜了一遍。theta_v0 里只剩测试断言、文档注释和 `bin/p119` 一处过期注释。其他命中都在旧线：`recursive_t/stream.rs:240,469`、`rec_stream.rs:157`、`orchestrator.rs:764`、`lib.rs:774,840` 的 `PyOnlineMacdState` 默认值、Python `src/newchan/*`。ADR 0039 表 `:222-223,:242` 把这些定为「冻结影子」，AUDIT.md:155 把它们归到 TB-10 #1368，所以不算本票范围。不过 PR 正文的排除清单没写 `orchestrator.rs` 和 `lib.rs:774/840`，建议补上。

**2. 到上界单独标 INCOMPLETE_RESOURCE_BOUND：PASS**
- 循环 `pipeline.rs:826-1652` 一共只有两个出口，我逐行查过，循环里没有 `continue`、`return` 或带标签的 break：
  - `:828-836`：单元数少于 `min_parts` 时退出，置 `stopped_naturally`；
  - `:1637-1644`：投影为空时退出，同样置 `stopped_naturally`。
- 循环外还有一个提前返回：`:686`，L0 为空时直接给 `Natural`，这是对的。
- `units` 的含义：每一轮末尾 `:1622-1624` 都会执行 `units = Rc::clone(&lc.projected_units)`。所以 `0..=l_max` 跑满、中途没退出时，`units` 就是第 l_max 级投影出来的单元，也就是第 l_max+1 级的输入。`:1654` 用它判断截断，口径正确。
- 这个判断也是精确的，不是放宽的近似。凡是通过循环顶部检查的一轮，都会无条件执行 `tower_snapshots.push`（`:1313`）和 `levels.push`（`:1578`）。所以「第 l_max+1 级输入单元数 ≥ `min_parts`」就等于「不设上界时输出会多出一级」，覆盖确实不完整。
- `stopped_naturally` 这个标记在 `min_parts≥1` 时其实多余，因为两种退出方式本来都会让 `units.len()<min_parts`。它只在 `min_parts=0` 的空投影情形下起作用，留着没坏处。

**3. 缺值时报错停止，不回退：PASS（附 note）**
- 记录缺失返回 `RecordMissing`；字段缺失、值为 `null` 或 `"AwaitingPolicy"` 返回 `AwaitingPolicy`；值非法返回 `Invalid`。见 `policy_records.rs:127-143` 和 `parse_header`。
- 不存在拿 `default` 字段顶替的情况：`value` 缺失时直接返回 `AwaitingPolicy`（`:133`）。
- `Default` 不算兜底。它读的是同一份记录，读坏了就 `panic!`（`config.rs:111,137`），没有第二个取值，符合 ADR 0041 裁定二的推论。
- `try_from_active_records` 先求出两个带 `?` 的值，再展开 `..ThetaConfig::default()`。所以记录坏了时会先返回 Err，不会在后面 panic。FFI 入口改走这条路，`ffi.rs:79-80` 返回 `PyValueError`。

**4. 负控和变异体：PASS-with-notes**
- 负控测试 `policy_records.rs` 的 `negative_controls_missing_unsigned_invalid` 共 9 例，覆盖了缺记录、缺字段、未签、null、负数、0 周期、fast≥slow、id 错、version=0，能把这几类情况区分开。
- 变异体只在一部分位置能被抓住：
  - 改在 `parse_*` 里的变异体会让测试变红：`reaching_l_max…`、`macd_record_version_flows…`、`other_legal_versions…` 至少有一个红，负控也会受影响。作者报的「3 红」我没复现，按推演是可信的。
  - 改在 `Default`、`active_*`、`from_active_record` 里、直接返回常数的变异体，所有测试都抓不住，因为常数和 v1 记录的值一样。生产路径（S 会话、各个 bin）走的正好是 `Default` 这一层。
  - 改在计算端的变异体也抓不住，比如把 `divergence.rs:157-159` 改成忽略 `cfg` 直接用 12/26/9。`macd_record_version_flows…` 只断言版本戳不同，没有断言读数变了。

**5. 等值对拍：PASS**
- `ClassifyOutput` 只 derive 了 `Debug, Clone, PartialEq`，没有 Serialize（`pipeline.rs:383`），所以不会引起任何 JSON 或 golden 文件漂移。
- 既有的 `==` 比较都是同一份配置下全量和增量对比。新加的两个字段是同一循环、同一配置算出来的纯函数，两边一定相等。
- 我 rg 搜过，没找到用 Debug 字符串当指纹、或靠配置是否相等来做缓存键的地方，所以 `LevelConfig`/`MacdConfig` 的 `PartialEq` 多了 `budget`/`record` 字段，不影响既有比较。
- 循环逻辑只多了一个 bool，分类、塔、候选、操作四份产物都没变。逐位相同的论证成立。

**6. 原文锚 025-第25课.md:16：PASS**
- 行号：`docs/chanlun/text/blog/025-第25课.md:16` 就是这段。
- 界内：`:54` 是 `---------↑正文---------`，16 在它之前。
- 行首无注：该行以「但“学如不及”」开头。
- 逐字：我用 Python 的 `find` 核了三段引文——`macd-force.json` 的 `source`、`options`，以及 `config.rs` 文档注释——都能在该行逐字找到，而且都在「(注：」之前结束。

**7. 其他：PASS（附 note）**
- 编译：`tower_policy.rs` 用到的 `classify`、`TowerTermination`、`ThetaConfig`、`operation_oscillating_layer` 都能从 `classifier/mod.rs:51,198` 和 `tests/fixtures.rs:104` 解析到。`LevelConfig`/`MacdConfig` 的结构体字面量构造只剩 `policy_records.rs` 和 `divergence.rs:1843`，后者已补上 `record`。没看到编译风险。
- 研究覆写：`lean_mirror.rs:47` 改用 `research_override_l_max(8)`，戳会写成 `research-override`，不冒充签署版本，做法合理。
- 和 ADR 0041 的四条落法对照：
  - 字段：在具名记录里；
  - 默认值：有，附出处；
  - 可选值和代价：有，塔预算写在字段级，MACD 写在记录级；
  - 改签：`version` 从 1 起，`active_records_carry_current_values_and_version_1` 把「值 + 版本」一起锁住，只改值不升版本会让测试变红。
- 命名跟 #1344 的 `second-seq-scan-budget/1` 是同一套格式。

---

**BLOCK 项（必须修）：无**

**Notes（可选，建议跟进）**
1. **版本戳可能和实际值对不上。** `LevelConfig.l_max` 和 `MacdConfig.fast/slow/signal` 还是 `pub` 字段（`config.rs:83,125`）。任何代码直接写 `cfg.level.l_max = 8` 时，戳仍然是 `tower-level-budget/1`，等于冒充签署版本。`research_override_l_max` 只防得住主动调用它的人。建议把字段改成私有并提供只读访问器，或者给 MACD 也加一个 `research_override_*`，再配一个检查：值和记录不一致时必须是 `ResearchOverride`。
2. **变异体覆盖缺口（第 4 条）。**
   - 在 `macd_record_version_flows…` 里加一条断言：v2 在该夹具上的 `classification` 或 hist 和 v1 不同。这样能抓住计算端忽略 `cfg` 的变异。
   - `Default` 层回退成常数的变异，目前在等值情况下测不出来。建议在文档里写明这个局限，或者加一个源码层守卫测试，检查 `config.rs` 里不再出现这几个数字字面量。
3. **下游还看不到截断标记。** `termination` 和 `policy` 目前只存在于 `ClassifyOutput` 里：`ThetaPiStream`（`stream.rs:120-122`）只取了 `classification` 和 `tower`，`PyThetaStream.finish_full` 也没导出。这两者在 ADR 0039 里是 OE08 冻结影子，S 会话现在也不消费级别塔，所以不阻塞本票。但专项验收①写的是「查询/报告能区分截断」，正式落地要等塔接进 S 输出之后，建议在票里写明这个后续，并说明和 #1488 的先后关系。
4. **另一份塔循环副本口径没同步。** `backtest/wverify_run.rs:3052-3053` 复刻了塔循环，注释还把「`l_max` 上界」算作自然终止。它属于冻结影子、只用于对照，但建议同步改注释，或者标明是旧口径。
5. **畸形字段被归到「未签」。** 比如 `{"l_max": 6}` 没有 `value` 包一层时，`read_u32`（`policy_records.rs:133`）返回的是 `AwaitingPolicy`，而不是 `Invalid`。仍然会报错停止，只是分类不够准。
6. **没测到「报错后没有产出」。** 负控只测了解析函数会返回 Err。记录是编译期常量，入口层没法注入坏记录，所以「报错停止且没有产出读数」这一段没有端到端测试。这是 `include_str!` 设计带来的局限，建议在 PR 正文里写明。
7. **几处过期注释。** `bin/p119_l5_decompose.rs:122,276` 还写着 `config.rs:82 l_max: 6`；`docs/reference-theta-v0.md:30` 也还没写达到上界要标截断。属于文档滞后，可以一起改掉。

## 第 2 轮定向复审（审 7fd1ac3f8c）

结论：PASS-with-notes

7 条 notes 都已处理，或者有合理的不改理由。这轮修订没有引入需要阻塞合入的新问题。下面的 note 1 和 note 5 各留一点小尾巴，都不阻塞。这轮仍然只读、没跑 cargo，测试通过以作者自报为准。审的是 HEAD `7fd1ac3f8c`，比上一轮多改了 5 个文件，共 +54/−5。

---

**note 1（输出戳写着签署版本，实际值却被代码改过）：已处理，有漏网路径**

守卫测试 `policy_records.rs:226-257` 做两件事：
- 检查 `config.rs` 里不再出现 `l_max: 6`、`fast: 12`、`slow: 26`、`signal: 9` 这几个常数写法；
- 扫描 `src/**/*.rs` 的每一行，先去掉 `//` 之后的注释，再找 `.level.l_max`、`.macd.fast/slow/signal` 后面紧跟 `=`（不含 `==`）的写法。

误报情况：
- 现有代码没有命中。我用 rg 搜了 `(level\.l_max|macd\.(fast|slow|signal))\s*[+\-*/]?=`，结果为零，所以守卫不会一上线就失败。
- 守卫自身的字符串不会被误判。`[".level.l_max", ".macd.fast", …]` 里每个模式后面跟的是 `"`，不是 `=`。注释行在去掉 `//` 后为空，也不会命中。
- `research_override_l_max` 里写的是 `self.l_max = l_max`（`config.rs:102`），不带 `.level.` 前缀，所以合法写法不会误报。
- `tower_policy.rs:67` 的 `cfg2.macd = parse_macd_force(…)` 是整体换成一份新记录，戳也跟着换，本来就合法；它也不匹配上面的模式。

漏网情况：下面这些写法都能把值改掉、戳却还是签署版本，守卫抓不到。目前代码里都没有实例，但守卫不能阻止以后有人这样写。
1. 结构体更新语法：`LevelConfig { l_max: 8, ..LevelConfig::default() }`。原因是 `budget`/`record` 字段是 `pub`，会被原样拷进新结构体。
2. 先拷贝到局部变量再改：`let mut lv = cfg.level; lv.l_max = 8;`。这里没有 `.level.` 前缀，守卫匹配不到。
3. 复合赋值：`cfg.level.l_max += 1`。守卫要求模式后面紧跟 `=`，`+=` 不满足。
4. `rust/tests/` 目录在 `src/` 外面，守卫不扫。我查过，目前该目录下没有这类写法。
5. 常数检查只看 `config.rs`。如果有人把 `policy_records.rs` 里的 `active_*` 或 `level_config_from` 改成直接返回常数，这一层抓不到。

根治办法还是第 1 轮的建议：把字段改成私有并提供只读访问器。作者选了守卫测试，能挡住最常见的 `cfg.level.l_max = …` 写法，可以接受。建议把这 5 条漏网写进 PR 正文的已知局限。

**note 2（计算端忽略配置参数的改动测不出来）：已处理**
- 新加的断言在 `tower_policy.rs:75-78`：用同一组价格分别按 v1、v2 参数调用 `compute_macd`，要求两次的 hist 不同。`compute_macd` 第 99-108 行如果改成忽略 `cfg`、写死 12/26/9，这条会变红。
- 第 1 轮我举的例子是增量算法的初始化 `MacdState::init`（`divergence.rs:157-159`）。这里另有既有测试 `incremental_macd_bit_exact_custom_cfg`（`divergence.rs:1838-1872`）兜底：它用 5/20/5 参数逐 bar 比较增量结果和批量结果，误差要求小于 1e-12。`init` 如果写死 12/26/9，两边结果对不上，测试会变红。所以批量和增量两条路径都有测试锁住。
- 这条新断言只锁到函数层，没有锁到 `classify` 的输出读数。按我第 1 轮的原意，这样已经够用。

**note 4（`wverify_run` 的旧注释）：已处理**
`wverify_run.rs:3052-3054` 把「自然终止条件」改成了「停止条件」，并写明这份对照副本不区分资源截断，生产口径从 #1603 起记为 `ResourceBound`。改法准确。

**note 5（格式写错的字段被归为「未签」）：已处理，有一处口径变化**
- `read_u32`（`policy_records.rs:133-137`）现在要求字段必须是带 `value` 的对象，否则判为 `Invalid`。新加的负控 `bare = {"l_max":6}` 断言结果是 `Invalid`，第 1 轮指出的问题已修好。原有的 `{"value":null}`、`{"value":"AwaitingPolicy"}`、缺字段这几例仍然判「未签」，没有回归。
- 口径变化：如果有人直接写 `"l_max": null` 或 `"l_max": "AwaitingPolicy"`（不包一层 `value`），以前判「未签」，现在判「格式错」。两种结果都会报错停下，不影响安全。但 ADR 0041:21 说未签字段就是 `AwaitingPolicy`，写记录的人很可能就这么直接写。我在仓内没搜到使用这种写法的 JSON 记录，也没有测试锁定这个分支。建议二选一：把这两种直接写法也归为「未签」，或者在 `read_u32` 的注释里写明「未签必须写成 `{"value": …}`」。

**note 7（过期注释和参考文档）：已处理**
- `p119_l5_decompose.rs:122,276` 不再指向 `config.rs:82`，改成指向记录 `tower-level-budget/1`。
- `docs/reference-theta-v0.md:31` 新加一条订正：写明记录路径、v1 = 6、到上界标 `INCOMPLETE_RESOURCE_BOUND`、MACD 读 `macd-force` 记录（出处 `025-第25课.md:16`）。内容和代码一致。文中的「≥3」就是 `min_parts_per_level` 的值，准确。

**note 3（下游还看不到截断标记）、note 6（没有端到端测「报错后没有产出」）：作者不改代码，改为写进票面或 PR 正文，理由成立**
- note 3：截断标记暂时只在 `ClassifyOutput` 里，要等级别塔接入 S 会话的输出后才会被消费，和 #1488 的先后关系需要写明。
- note 6：记录是用 `include_str!` 在编译期编进程序的，入口层没法塞进一份坏记录来测。这是设计带来的局限。
- 两条都不需要在这次修代码。只是 diff 里看不到票面和 PR 正文的改动，我没法核实它们是否已经写上，合并前请确认。同一处还要补上第 1 轮 note 1 提到的排除清单：`orchestrator.rs:764`、`lib.rs:774/840`。

**修订是否带来新问题：没有阻塞项**
- 守卫测试会在运行时遍历 `CARGO_MANIFEST_DIR/src`，范围不含 `target/`，开销可控。
- 它按 `//` 截断每一行，含 `//` 的字符串（比如 URL）后半段会漏扫，但不会造成误报。
- 每个模式每行只找第一处出现，同一行写两次会漏掉第二处，概率很低。
- 唯一的行为变化就是 note 5 说的口径变化，两种结果都会报错停下，方向安全。

---

**剩余 notes（不阻塞）**
1. 把守卫测试的 5 条漏网写进 PR 已知局限，或者改用私有字段从根上解决。
2. `read_u32` 遇到直接写 `null` 或 `"AwaitingPolicy"` 的字段，现在判「格式错」，要么改回判「未签」，要么在注释里写明正确写法。
3. 合并前确认票面和 PR 正文已经写上 note 3、note 6 和排除清单。

## 作者处置

- 第 2 轮剩余 note 2：已改，直接写 `null`/`"AwaitingPolicy"` 的字段判 AwaitingPolicy，另加负控一例（后续 commit）。
- 剩余 note 1（守卫漏网 5 类）、note 3（下游尚未消费截断标记）、note 6（include_str! 无法端到端注入坏记录）、旧线排除清单：写入 PR 正文「已知局限」与票面证据。
