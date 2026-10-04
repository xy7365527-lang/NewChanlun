# Stage47 直接路线实际输出桥：作者自检

本报告归属 #1467，名分为研究工作草稿。原冻结目标与实际历史支撑补充目标均已取得 Lean exact root；**仅为作者机器自检，独立语义冷读与独立验收待完成**。没有改动旧 Spec/Proof、`formal/`、生产代码或 `ResearchProgress.md`，没有提交、推送或发布 GitHub 评论。

## 冻结合同与数学范围

原合同先冻结为 `DirectOutputSpec.lean`，随后才编写 `DirectOutputProof.lean`；四项声明没有弱化或追加前提。

1. `SOURCE_TARGET`：对原 `MovingQuote.construct es qs = some out` 的全部输出，`out.map located` 精确等于原 `locate 0 (group es)`，解码为原 `es`。每个输出绑定一个前缀、自有事件块和后缀；前缀长度是 `start`，`finish = start + 自有块长度`，自有事件全部同号；同时满足既有 `LocatedProof.Good`。`knownAt = some (finish+1)` 当且仅当该原符号块满足 `Completed`。
2. `COVER_TARGET`：每个原事件索引 `i < es.length` 都属于恰一输出值的半开区间 `[start,finish)`。量词覆盖全部输出，包括 `geometry = none`。这里声明的是索引归属的输出值唯一性，并未另以 `List.Nodup` 为目标。
3. `HULL_TARGET`：假设原书路径 `BookQuote.Through es bs es.length`、原报价读取 `BookQuote.Reads bs qs`、`qs.length = es.length+1` 及原构造结果。任一实际 `geometry = some s` 都绑定原 `qs[start]`、`qs[finish]`、原两端 `bs`、自有事件及同号 `Flow`；`s` 就是这些端点生成的原 `leg`，Stage46 `DirectHull.Support` 的精确外包络为 `segLow s`、`segHigh s`。此声明覆盖末尾尚未确认的块，强于仅已确认输出的目标。
4. `ADJACENT_TARGET`：对未过滤原输出序列中的直接相邻两项，前项 `finish` 等于后项 `start`，符号相反，二者共用同一个原 `qs` 切点读取。如果两项几何均为 `some`，端点价格相等。没有删除 `None` 后再跨接。

`Completed` 沿用既有“连续同号块已被反号事件确认”含义，不表示缠论原义走势已经完成。`ExactHull` 表示两端在支撑内且界住全部支撑，不额外声称支撑集合等于端点间的每一个整数点。

## 实际历史补充合同

原 `DirectHull.Support` 量化与两端兼容的 `Flow` 分裂，不能单凭其类型称作指定 `bs` 的内部历史。收到这一边界提醒后，另冻结 `DirectOutputSourcePathSpec.lean`，另写 `DirectOutputSourcePathProof.lean`，原四项目标保持不变。

`ActualSupport bs qs start finish z` 明确要求：存在实际原切点 `k ∈ [start,finish]` 和报价 `q`，原 `qs[k] = some (some q)`，`QuoteOf (bs k) q`，且 `q.bid ≤ z ≤ q.ask`。`ACTUAL_PATH_TARGET` 在与原包络桥相同的输入假设下证明：所有这些原切点确实可读，且**这条指定 `bs/qs` 历史**的 `ActualSupport` 精确外包络就是实际输出 `s` 的 `segLow/segHigh`。

证明从原 `readout = some` 提取窗口全可读性，以原 `Through` 的每个 `Step` 和原块的同号性逐步证明实际报价链单调；再证明内部支撑被两端界住，且端点价在实际起止切点达界。不是用另一分块器或另一条同端点路径代替实际历史。

## 作者机器回执

仓外证据根：

`/Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output/`

| 根 | exact / closure | run ID | semantic |
|---|---|---|---|
| `DirectOutput.exact_root` | `true` / `closed` | `4c549c507c87401a820ba9d396176201` | `not_reviewed` |
| `DirectOutput.actual_exact_root` | `true` / `closed` | `4c6213d2f1484e03888ccfb92bff7469` | `not_reviewed` |

两根的期望类型、binder kinds、universes 均匹配；传递公理闭包均只有 `propext`、`Classical.choice`、`Quot.sound`。`unexpected_axioms`、`unsafe_dependencies`、`unknown_dependencies` 均为空。加载模块数分别为 2279 与 2281。完整导入库存和日志留在各自 manifest 及不可变 run 目录；这里不复制约数 MB 的 manifest。

| 文件 | SHA-256 |
|---|---|
| `DirectOutputSpec.lean` | `23d38c0d49b57d7438efd90757dc7a80e138d54400de123955280857164c8e6b` |
| `DirectOutputProof.lean` | `a891306d2e94fe60e06acdde4bb1806f37e6f71460147f4fa095d8f77b187f2a` |
| `DirectOutputSourcePathSpec.lean` | `9cccbdf9b546c51aa3eaf323f282ea81a58032d3bb15c889c18efdc17bf9d648` |
| `DirectOutputSourcePathProof.lean` | `531a46969a849d5e6641a7685c398404366a308b7135e9eed79b0f3c1e8cba23` |
| `author-v1/run-manifest.json` | `1df391eae1fff6129987d6020de8795d4a0e776a8bbdde18303e5174b620365a` |
| `author-actual/run-manifest.json` | `ebb596b10b822f6c52729bc5c69332ecaa8d7270da9d8f2345e7456a2752c491` |

原根语义身份：`a64c15a8f4d26d0c08fdb1595ec656b835fe4d2e3aa239312bcc1920a8eef5a1`；实际路径根语义身份：`67c2f30066c85bdcc74c04fc0fae51376874b0bb29dfeb9353e8fe1641f7939b`。

工具链：Lean 4.31.0，commit `68218e876d2a38b1985b8590fff244a83c321783`；Lake `5.0.0-src+68218e8`；Python 3.14.6；macOS 26.6.2 arm64。完整可执行文件、验证器、合同和环境哈希见 `versions-and-hashes.json`，全部源哈希见 `final-source-hashes.json`。初始 `frozen-inputs.json` 逐项复核未变，仓外 project 源与仓内对应源逐字一致。

复现命令（输出目录使用新的路径）：

```sh
cd /Users/silencehan/Documents/Codex/research-evidence/issue1467/turn-semantics-v1/stage47-direct-output
./verify.sh independent-v1
./verify-actual.sh independent-actual
```

两个脚本都显式指定 `--contract`、Lean/Lake 可执行文件、`--direct --build-timeout 60 --strict-exit`，并设置原研究树 `formal/.lake/build/lib/lean` 为 `LEAN_PATH`。回执消费也必须保留该环境；首次不设置 `LEAN_PATH` 的消费按设计返回 `runtime_search_path`，该失败回执保留，不能当作数学失败或忽略环境差异。

设置原 `LEAN_PATH` 后，两根 `lean_evidence.py` 消费均返回 `status=current`、`snapshot_current=true`、`exact_root_passed=true`、`reasons=[]`。回执分别保存在 `author-v1/freshness-recheck-with-recorded-path.json` 和 `author-actual/freshness-recheck-with-recorded-path.json`；消费检查不等于新的内核重放。

## 尚未越过的边界

本结果建立原输出与原事件、书、报价以及 L1 精确外包络的普遍联系。没有证明任意原事件价格或全部深层库存都在该 L1 包络内；Stage46 的深档反例仍成立。没有证明原义走势、递归生产接入、策略优势或盈利。独立语义冷读、对照合同及独立机器回执由验收方完成，作者不代签。
