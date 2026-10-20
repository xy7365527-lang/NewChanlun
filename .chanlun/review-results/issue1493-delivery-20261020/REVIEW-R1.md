**VERDICT: PASS 附必改**

审查对象：分支 `codex/1493-segment-turn-20261020`，`git diff 36adc7c7c8..HEAD`，HEAD 在审查中推进至 `17c7e24e08`（已按新 HEAD 审）。

## 发现（按严重度）

**必改 1 — 裁定验收清单要求的「OPEN 封段对照见证」未加**
`rust/src/theta_v0/parser/tb02w_tests.rs`（本轮 diff 仅改 W1 期望，无新增测试）。裁定 draft.md:62 明文：「W2 已是预算封段见证，加一条 OPEN 封段的对照见证」。本轮新增的 11 个测试里没有一条直接构造「同段第二种候选 OPEN 之后再来一个旧规则下会成立的候选」并断言不在该处终结；裁定 1 目前只由 W1 `diverged.is_empty()`、`cascade_revival` 单调、随机 full≡incr 间接覆盖。改法：在 tb02w_tests.rs 加一条构造性测试——候选 b 的第二序列无分型且预算未耗尽（`semantic_state == Some(OpenAtPrefixEnd)`），再追加能形成更晚候选的笔，断言全量与增量都不在更晚候选处封段，且输入笔数越过 `TAIL_WINDOW=7` 后仍不封段。

**必改 2 — 验收清单第 4 条（三方对照与 3 组手算写进 #1493 评论）未完成**
#1493 今日评论止于裁定本身（6282247099），没有作者的对照读数与手算评论。读数：随机 802 组差异 [3,262,263]，裁定臂 endpoint≠P 0、门失败 0；真实数据 FULL 段数 [51741,51417,51825]，parse_invalid 全 None。我的独立手算（可引用）：rand0/true seg#8（Up，a=s57、b=s59 有缺口、s61⊂b 合并保 b、s63 封口先破确认，段端 s58=11607059=P；旧 G-2 臂段端 s60=11513061≠P）、rand1/true seg#35（Down，b=s216、s218 合并、s220 封口，段端 s215=8823216=P）均与裁定臂一致。

**必改 3 — HEAD `17c7e24e08` 无测试收据**
该提交改 `segment.rs:1318-1319/1335-1336/1348-1349`（append 三个早返回分支的 `parse_invalid_from` 与续扫路径 :1369 同一过滤），逻辑我核过是对的（#804：无确认段且无可扫种子时全量同输入亦无门失败），且 `tb02c_tests.rs:355`、`tb1493_replay_tests.rs:115` 的逐前缀 `parse_invalid_from` 全量≡增量断言能锁它。但远端日志只有 `b698ce1d96` 的全量收据（`cargo test --lib` 2864 过 0 败、FMT_RC=0）。改法：对 `17c7e24e08` 补 `cargo fmt --check` + `cargo test --lib`（或 CI dispatch）并贴进 #1493。

**必改 4 — §15 行号漂移（docs 提交 `8bf1681205` 落在后续测试提交与 fmt 之前）**
`.chanlun/definitions/xianduan.md` X-33：`tb1493_replay_tests.rs:60/:154` 现为 `deep_unverified`、`r.emits`，不是所述断言；X-34：`tb1493_replay_tests.rs:461/:502` 现为格式化行，真实位置 `ruling2_arm` 约 :480、`tb1493_ruling2_three_way_real` :521、`tb1493_ruling2_real_endpoint_is_fractal_p` :559。其余 X-33～X-36 行号（feature_seq.rs :437-451/:511-516/:692/:704-720/:861/:992-1001；segment.rs :1145-1148；tb02c_tests.rs :678/:710/:756/:802/:914/:939；s_tb02_contract.py :47/:993；q3-typed.js :187-188；测试 py :161/:178）均指到正确代码。改法：按 HEAD 重标两处。

**观察（非阻断）**
- 「段端非本段极值」计数：裁定臂 922/33 组高于窗成员臂 792/32，测试只打印不断言，作者按「极值＝分型 P」口径解读。建议在 #1493 评论里写明读数与解读，免得被当作裁定 2 引入的退化。
- xianduan.md §7.1 第 1 条新增括注「右侧元素未到是任何分型都要等的，`065:28`」：行号正确（原文「被分析的K线已经走出来」在该行末段），但引号内是转述，建议改成原文短语。

## 各项核验结论

- **A（裁定 1）**：OPEN 与预算耗尽是 `feature_seq.rs:992-1001` 同一 match 臂、同一 `unresolved_blocked`、同一 `return None`；`scan_trigger` :861 在尾窗起点计算前早退，尾窗无法挪出；全量 `segment.rs:990` 与增量 :1396 都以全 strokes 切片调用同一状态机，增量每次 append 新建（:1373）；KILLED 末笔失效仍 `continue`（#1420 既有）。被改测试诚实：`cascade_revival` 改为增量≡全量＋已发射段逐前缀单调；W1 保留 `euf==Some(69)`、改 `diverged.is_empty()`；replay 把「只有末笔变且段起于 euf 之后」归末笔可改写，其余计违例并断言 0。
- **B（裁定 2）**：:662-730 分派穷尽（无缺口 c⊂b→窗；有缺口并列→窗；有缺口 `left_inc`→merge 且 `keep_b` 不改 `stroke_idx`；非峰→R3）；record_step merge 的 `after.stroke_idx` 取 `std.last` 保 b；`GAP_INCLUSION_ARM` 仅 `cfg(test)`，`cfg(not(test))` 恒 0。
- **C（裁定 3）**：`later_failures==0`、`later_min_count==0` 为真断言；合成 K 线锁逐字复制 #1581 生成器并走 `ParseLayerIncr` 生产路径，6 seed×4000 bar 全程无标记，有意义。
- **D（裁定 4）**：旧 `tb01c-client.js`@2331440468 的 v4 校验（`turnCandidate`、`turnLinks`、窗成员元素不变式、`tie_with`/`window_member` 步、checks `turn_hypothesis_state`、`open_turn`/`killed_by`、RULE_CONTRACTS v4、SECOND 终态绑定、parse_invalid→CC-056 与逐轴阻断）在 HEAD `q3-typed.js:294-490` / `q3-consumer.js:1421-1447` 逐项齐全，函数体 diff 仅为缩进，外加新门 :483「段结果未归签名目录叶」。旧有而新无的 77 条错误信息全部是 #1394 在 main 上重构的非段校验（main 6badaa6049 亦无），本轮无丢失。归叶表 Python :47 与 JS :188 同一（PENDING_CASE_ONE→NO_FIRST、KILLED_CASE_TWO→PENDING_CASE_TWO），由 `test_result_leaf_table_covers_v4_results` 跨语言锁住。
- **E（正本）**：§5.1/§5.2 订正/§7.1 第 1/3/4/6/7/8 条/§10/§15/§16 V-15、V-17 与裁定文字一致；引文抽查 071:38、078:52、071:32、062:26、077:62、065:28 均对上 `docs/chanlun/text/blog/`；X-37 指向 #1581。行号漂移见必改 4。
- **F（merge 冲突解）**：`RULE_REVISION` 取分支值 `+segment-turn/1`、注释两侧合并；`CatalogAxisTable::for_rule_revision` 补 `+segment-second/2` 只读 17 轴并同步旧规则测试列表；index.html/tb01c-client.js 先取 main 版后按新结构移植（D 已核）。无丢语义。

## 跑过的检查与读数

- `node s_session/tests/build_tb01c_browser.cjs --check` → passed（runtime 90087f12…、bundle ec0ad2db…）
- `node --test test_tb02x_contract.cjs test_tb02c_contract.cjs test_tb02w_contract.cjs` → 14 pass 0 fail
- `python3 -B -m unittest test_tb02c_turn_contract test_tb02w_contract` → 31 tests OK
- 远端收据：`b698ce1d96` `cargo test --lib` 2864 过 0 败 149 ignored，FMT_RC=0；`e0b57c2d66` replay ignored 3 过（全量 4,613,598 根）；`17c7e24e08` 无收据
- 校验信息集合对照：旧 304 条 vs HEAD typed+consumer+bundle 并集，段相关零缺失
- 本机未跑 cargo，未改任何文件。
