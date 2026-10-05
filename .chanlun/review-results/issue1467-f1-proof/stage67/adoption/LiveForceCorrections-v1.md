# Stage67 进行态力度采用订正 v1：撤回读取隔离声明

2026-10-06，研究票1467。只新增本文件及`live-force-corrections-v1.json`，保留live-author全部冻结字节、原online-seal与失败字段；不重跑历史或修改脚本。依据[最终独评](../live-review/review.md)，M1是本次唯一HIGH必修。本版为作者采用订正，待采用复核，不将原包改称无保留通过。

## M1：实际发生过的读取必须照实记录

原`live-author/Report.md:23`称最终pens/objects/parent在结果封存后才读取；`check_live.py:212`静态输出`online-seal.json.final_pens_read:false`。**若这些句子/字段表示seal前没有读入最终笔信息，该声明不成立，现明确撤回。** 不能仅把旧字段改个名字就声称它从来正确。原字段作为被独评驳回的记录保留。

冻结脚本的实际顺序如下。

| 顺序及代码位置 | 实际动作 | 可以与不可以推得什么 |
|---|---|---|
| 1，`:38`经`:14–21/23–28` | 对INPUTS全部文件流式读取字节计算SHA256，已包括最终independent-pens/objects/parent及完整reference | 这是字节完整性预检；不是JSON语义消费，也不是“还没读取文件” |
| 2，`:39` | 整体反序列化source.json及443项independent-prefixes.json | 整个进程已载入完整历史/前缀数组；不得声称物理上只持有当前及过去数据 |
| 3，`:40`调用`:31` | 对reference.stdout整体执行`json.loads`，顶层`groups`、`prefixes`、最终`strokes`均进入内存 | 最终笔信息确在seal前被反序列化；`:41`显式只比较prefixes及数量，不等于其他字段未被读入 |
| 4，`:43` | `del reference`删除该变量引用 | 不撤销之前已经发生的读取，不能恢复读取隔离 |
| 5，循环至`:197` | 在固定模板中按t计算，计算操作数取当前prefix、已经加入seen的观察及先前证书 | 可以检验具体计算的数据依赖；这与进程是否曾持有其他信息是两个问题 |
| 6，`:208–212` | 保存2215行与派生表，计算结果SHA256并写online-seal | 输出内容在此封存属实；其中无读取终值的静态布尔声明不因此取得证明 |
| 7，`:214` | 才单独反序列化independent-objects.json、independent-parent.json及independent-pens.json，作闭合对照 | **这三个单独文件的JSON解析后于seal**属实，但不能排除步骤3已经从reference读到的同类最终笔信息，也不否认步骤1的字节散列读取 |

因此，原`final_pens_read:false`的强信息隔离解释登记为**rejected**，采用事实为`reference_final_strokes_deserialized_before_seal=true`。`final_objects_read:false`也只能按“尚未反序列化两个具名independent对象文件”的窄语义解释，不能当作未来对象信息没有进入进程的全局保证。文件后读、内容早读、散列读字节和输出依赖分别记账。

## 替代采用文字

> 在预先冻结的bootstrap加四组5×5模板及已审前缀输入条件下，作者循环的逐时计算操作数由当前prefix、已经到达的观察及证书构成。独立重写核对了全部2215行，并以仅保留prefixes的reference对照复现了八份核心产物；这些证据支持本次有限输出不依赖reference的顶层最终strokes/groups。原作者程序在online-seal前实际整体反序列化过含这两个字段的reference，且预载完整source/prefix数组，因而没有实现物理上防未来信息的读取边界。其三个单独independent最终文件的语义解析确在seal后，这一较窄读取事实保留。

该替代文字覆盖Report:23及原online-seal的强隔离读法。`no_endpoint_backfill`等原字段仅保留其已检查的计算依赖含义：没有用最终对象结束减长度来选当前c起点；不扩张成“最终信息未被加载”。若将来需要强读取隔离，应另立执行器版本，使在线阶段不装载终值参考/未来数组，并核该新版本。当前不改旧脚本，也不把独评的对照执行器追认为原作者执行器。

## 独立证据支持的有限范围

独评`independent_check.py`没有导入或执行作者函数；其在线seal之前不读reference或最终独立对象文件，`Auditor.step`只接收当前观察和当前prefix，独立产出2215行，逐字段与作者相同，差异数0。该驱动仍预载source数组，因此独评本身也把step数据依赖与整个进程的预读区别开了。

独评另做原样reference与只提供prefixes两次顺序回放，均exit=0；[replay-comparison.json](../live-review/replay-comparison.json)记录以下八项与原作者冻结SHA256逐字相同：prefix-results、role-first-known、first-triggers、online-raw-certificates、active-reanchoring、endpoint-comparison、S15-narrow-status、whole-certificates。这里**引用已经完成的独评运行，没有再次运行历史**。

这些结果不证明任意历史、一般R_W解析器或一般原义选臂的因果性；也不补出Residual结构笔资格，不认证GeneralDiv、Completed、Owner/Next或固定F₂。因果性表述继续限定于已选模板、已审前缀和实际计算依赖。删除两个参考字段的有限不变性不能否认M1的历史读取事实。

## 数值及未决状态保持

A在P0/P1/P2/c0末c/父完整c0的旧闭合端点L仍为−900、50、−350、175、875，五项均不等旧值；R仍为−500、−1200、−200、−4200、−3500，五项有限匹配。bootstrap b_low闭合L=−1000、完整c0闭合L=−3500，不混成末c的−4200。

局部角色门仍为118/218/318/418及父338；A首次联合候选仍为118/无/318/无/父422，R仍为118/220/318/420/父421。父A的422超过冻结c0终点421，Owner/Next缺口照旧；早触发不改原对象边界。下行仍带号直接比较，事件时长end−start不混入生产inclusive根数。20窄证与S15选定源证的有限范围不变。

原义source-role、StructuralPenNow、GeneralDiv、Completed、Owner、Next、固定F₂仍未建立，相关字段保持null；完整初始化与每时刻总性仍未证。P1–P4、R_W/R_D及原验收不变。本订正只纠正证据声明，不新造语义资格。

## 冻结与本版状态

原作者manifest `e0dc6fd6b6f4221bac4d9b6e187d75ab0c54f48a88ac8d84f64bc6ff75a87581`及FINAL `8b3c94b16c3aae2968fa361568c4c221ac9653ef6a23a469193fa4a9c3589c54`保持；本版对live-author全部36份文件作前后哈希检查，没有改动。机器订正记录包含旧字段驳回、实际顺序、依赖边界、独评哈希及冻结复核。状态为`M1-corrected-in-adoption-layer-awaiting-review`，不是原作者包无保留通过，也不是整个研究完成。
