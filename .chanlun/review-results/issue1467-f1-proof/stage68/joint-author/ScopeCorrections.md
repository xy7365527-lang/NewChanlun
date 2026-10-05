# 作者采用订正：所有路径与maximal路径

保留唯一主运行的全部字节，不再次执行check_joint.py。作者辅助Python审查发现，冻结卡Rδ写所有边路径，主程序却只输出maximal路径。该差异必须明示，不能由最长路径唯一宣称所有δ唯一。

本有限向前无环图中，每条路径都能延伸至一条maximal路径。因此对已计算maximal路径取全部前缀，恰得到冻结Rδ的全部路径。`complete_path_scope.py`只对既有输出作这一闭包，产物为`all-delta-paths.json`，未重新生成/检验whole候选，没有新增历史或搜索。

238起，全部申请关系中有两个非空表达式：

- `[W68:S1-S5]`，活动尾E122…t；NoPP=true，仍缺三孩子。
- `[W68:S1-S5,W68:S6-S10]`，活动尾E222…t；NoPP=false，也缺三孩子。

另有空路径，不能作成功。因此所有δ申请不是唯一。它们不是两份已认证原义合法分解，不能直接把申请多解说成原义唯一分解反例。该候选若要求只剩唯一合法规范δ，仍须补充分解准入/活动尾规则；本轮不加事后tie-break。

`model.json`及`result.json`中相关“唯一／未发现多路径”字段按**maximal路径**的窄范围采用，完整关系以本覆盖层及all-delta-paths为准。主结果238后的NoPP通过路径数0也仅指数maximal路径；短的一因子前缀通过NoPP，仍无三孩子，不能作为无限Pending逃避非空上推要求。

所有路径最长仍只有2项，故原40窗口计数、四条边、无三孩子、去NoPP仍无三孩子与5+9+5条件无解证均不受影响。完整原义合法性和唯一性仍unknown。

另保留`InvocationCorrections.json`对cwd的独立订正；实际进程工作目录为R，原回执中的HERE仅是输出目录。两项均属作者采用订正，尚待独立审查，不称原包无保留通过。

## 完整性边界及三类路径

`path-classes.json`分别列全部edge路径、非空NoPP通过路径及三孩子路径。238后依次为3条（含空）、1条、0条。短路径[W1]的活动尾明确包含已可供选择的W2；它不具maximality，不能宣布已完成整个初始分解。即使maximal路径的尾内也可有无法接续的后续候选，覆盖所有事件不等于这些事件已取得完整原义分解。每条路径另标extendable_now及candidate_windows_inside_tail；complete_original_decomposition保持unknown。全部候选申请与来源合格的完整分解分开，未用短路径永久Pending充成功。
