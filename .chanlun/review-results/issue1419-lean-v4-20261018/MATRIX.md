# #1419 11 项来源与适配矩阵

每行的基线/donor/最终 SHA256 及字节数见 `SOURCE-MANIFEST.json`；原始准备 INDEX 的 11 项均在写入前核过原字节与精确 patch。来源固定为 `634f62cddfaddf656a35ca9b9db20cb716b62bff` 的同路径，基线固定 545e。本轮只逐项采用已核对的候选义务；没有复制 Rust 或改变它的 API。

| 路径（相对 formal） | 来源保留部分 | 相对 donor 的必要适配 | 同拍/证明义务 |
|---|---|---|---|
| `Formal/RecursiveConstruction.lean` | 全 subs 互递归区间、空列表 `(0,0)` | 加 centers/level 无关引理及正负价、空与嵌套例 | RMove.lo/hi、LeveledMove 缓存值；数学模型已核，编译/产品未跑 |
| `Formal/DivergenceNesting.lean` | currentExtreme 读取 Move.interval；正价下行例；sampleOpenTail 实际 subs | 与 donor 字节相同 | 消费者从中心外缘转为走势区间，pending 契约不改；证明体待类型检查 |
| `Origin/PromQual.lean` | shiftMove_interval / shiftMoveList_listInterval 互递归证明 | 与 donor 字节相同 | 只改级别不改区间，后续 lo/hi 与 windowCenters_shift 消费保持；待类型检查 |
| `Origin/SegmentFeatureSeq.lean` | FirstBreak、TurnWindow、firstOutside、首出窗 iff 与追加保持 | 加双方向同笔越过两端优先定理；具名说明当前 /2 未接新窗口 | 不省略无并列/首出窗前提；第一种新极值优先；不更改旧严格谓词作宽档兜底 |
| `Phase2/Claim10_SegmentV1.lean` | 第二终结静态三态、终结 iff confirmed、订正 071 引用 | 撤回已三路锁定的暗示；列出 D_SECOND_FOLD_REFINEMENT 状态接口和预算/恢复依赖 | 同笔包含/分型优先，原始 reach 后判失效；动态定理明确未实现 |
| `Origin/CenterConstruction.lean` | isZn、znMax/Min、computeGG/DD 与外缘包核心证明 | 顶层算法说明改为 Zn；020:52 改为查过的逐字短引文，seed 方向明确归已批 D-A | 全三段严格核心、Zn 外缘；方向不交替判别例；不认领完整构造桥 |
| `Origin/CenterConstruct.lean` | 方向投影与 Zn 聚合，C 续扫，4 工作例与导出字段 | 加非 Zn 聚合不变及成员推进引理；新 native_decide 改 decide；撤去缺失 S0 独立来源/当前 Rust 消费 scope 的断言；修参考骨架说明 | ≤8 成员与生产扫描源码对应；旧 ref_v1/center fixture 具体缺口保留；不扩到 ≥9 |
| `Origin/CenterStates.lean` | 五叶 iff/唯一、旧三态投影及公式等价 | 判定顺序改与 545e 一致；补四态类型/投影/谓词 iff/唯一证明体 | 有序包核域；不把几何相切记延伸，不把扩展当上级已构造 |
| `Origin/CenterPairFixtureExport.lean` | 8 例输入、schema、五叶/三态输出格式 | 撤 oracle_pairs.py 独立生成断言，绑定固定 donor/current fixture、NOT_RUN 与无矩阵导出边界 | 数值输入/模型标签与 JSON 静态一致；实际导出及 Rust 消费待验 |
| `Origin/ScanAssemblyMirror.lean` | donor 的 3 行四态域注释 | 与 donor 字节相同 | 仅说明，零新实现/定理；非正式五叶→镜像精化桥 |
| `Origin/UnifiedScanMirrorBridge.lean` | donor 的 3 行四态域注释 | 与 donor 字节相同 | 同上，保 #1429 有效域边界 |

数值来源与执行身份分离：C/D 的小型反例是本轮有限数学模型；E 四例数字来自固定 donor 的 Lean 字面期待值；F 八例来自同 donor 导出器且与现有 Rust JSON 一致。没有一项被标为本轮 Lean 已求值或独立 oracle 已恢复。旧消费者和导出漂移的待办不是更换判据的许可。
