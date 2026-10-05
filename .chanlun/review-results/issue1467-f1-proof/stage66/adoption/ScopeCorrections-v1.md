# Stage66 采用范围订正 v1

2026-10-06；研究票1467。作者采用订正，回应独立嵌套审查已交出的R1/R2。独评最终报告尚可能追加发现；本文不宣称独评通过。只新增本文件及对应corrections-v1.json，nested-author冻结字节全部保留。

## R1：撤回c0直接构成K1延伸的推断

原位置：nested-author/Report.md:82，以及:78、:99对“完整中心读法失败”的概括；nested-author/admission-and-lifecycle.json:40–73的center_lifecycle（特别是:60 extension_at_local_known、:61 extension_test、:62–67 current_member_table_under_full_extension、:68–71 current_outer），及:74–95的条件所有权检查。

错误前件在020:52：Zn指与中枢形成方向一致的次级别走势。当前K1初始成员为Up/Down/Up，形成方向Up，c0方向Down。因此c0整片与K1核心有交集这一数值事实，不能直接作为020:56的Zn实例。此前只检验区间相交便推进生命周期，漏掉了同向前件。

**替代采用文字：** c0 whole=[37800,72800]与K1 core=[62000,72000]的交集确为[62000,72000]，但c0不是这里的同向Zn。现有论证不能推出c0应进入延伸、延伸在438发生/可知、完整中心材料已经包括c0，或当前外缘/DD已经变为37800。本轮对这些生命周期结论不予采用；也不从这次误用反推“所有其他合法延伸均不存在”。

**机器字段采用覆写（原JSON不改）：** center_lifecycle.extension_at_local_known、extension_test、current_member_table_under_full_extension、current_outer均撤回，采用值为null、状态为not_established。若保留原extension_test=true，只能重命名并降格为interval_overlap=true，不得仍叫原义延伸判定。seed_members、seed_core、fourth_whole、intersection这些输入及交集数值不受此订正影响。原conditional_on_original_child_qualification=true不足以救原推断：即使另外证明孩子完成，同向Zn前件仍未满足。

条件算术单独保留：**若另有依据给定**FullCenter Own=E22…421，并在同一不重计表达式中另加c0 Own=E322…421，则重叠恰为100事件。这只是一条带明确假设的区间算术；本轮没有取得该FullCenter Own的源义前件，不能据此说实际D1重复计数、违反规范构造或已经被反驳。Report:82的“把K1偷换成完整延伸中枢会重复Own”采用时必须改成上述条件命题。

## R2：区分选定接口不满足、执行证据与语义未建立

原位置：nested-author/Report.md:5的“H_b实际返回false”、:76的“admission-and-lifecycle.json执行的类型检查”、:78的“拒绝guard”，及:99概括；nested-author/admission-and-lifecycle.json:3–18的first_actual_submitted_model_failure，特别是:12–16的tag、null和typed_gate_value=false；nested-author/author-receipt.json:7的admission_lifecycle退出记录。另见source-supplement/actual-relations-v2.json:609的parent.original_completed=false与:615的parent.local_completion.typed_completed_original=false。

冻结build_check.mjs和source-supplement.mjs均未实现、调用H_b准入guard。admission-and-lifecycle.json中的false是作者直接写入的记录，不能作为这两份冻结脚本执行了该guard的证据；author-receipt的admission_lifecycle=0也不补出冻结执行器。撤回“作者冻结程序实际拒收”“最早可运行准入反例”等执行层表述。

**替代采用文字：** 作者所选接口要求b满足completed-original-motion身份和已给出的原义完成证据；本次独评把b_low的实际raw-067-segment标签及GeneralDiv/Completed=null明确代入该接口，得到该接口的证据要求不满足。这个false属于“作者接口的当前证据准入”评价，不是原义GeneralDiv=false或Completed=false。原始段21/38的结束证及本轮数值照常保留；低阶原义完成关系仍是未建立。

本次独立代入记录见nested-review/H_b-evidence-versus-semantics.json：reviewer_evaluated_author_evidence_guard=false、author_executable_guard_found_in_two_scripts=false，且明确null_implies_semantic_false=false。此处引用的是已交付的独评计算工件，不以其存在宣称最终审查通过。

**机器字段采用覆写（原JSON不改）：** first_actual_submitted_model_failure改读为author_selected_interface_assessment；typed_gate_value=false改读为reviewer_evaluated_evidence_interface_satisfied=false；原义GeneralDiv与Completed维持null，语义状态为not_established。parent.original_completed=false不采用为原义否定，采用值为null；typed_completed_original=false只保留为当前证据未获原义准入，不能推出语义不完成。original_certified=false及original_completed_certified=0仍可保留为“本包没有认证完成”的证据计数。

035:22允许较低级走势作为前导，084:52/54/56允许另设f1且不要求无限前置递归；两者与作者接口的字面tag要求分开。原文并未要求这个tag，不能将所选接口不满足推广为一切低阶b都不合法、所有F1都不存在，或将当前合法数量历史逐出输入域。另一方面，这两处自由也没有自动给出当前候选的完整源义接口证明。应报告“尚未交付共同解释”，而非“已证明源义不可能”。

## 依赖结论与保持范围

父局部候选与规范构造资格仍须分别记账。原报告不能再以R1生命周期推断或R2的假执行叙述，将“未取得完整F1/固定F2资格”加强为“实际D1已被反驳”。本轮没有改变既定NoPP的作用域，也没有把MemberNext提升成任意规范Adj。

S15选定边界补证、443前缀、110笔/EOF稳定109笔、P0/P1/P2/c0局部证齐138/238/338/438、父力度−1000/−3500、父局部发生421与证齐438先维持原有限范围；本订正不重跑、不新增历史，也不自行扩大其源义认证。b若另获q0完成资格，b/P0/P1于238的更早三交诊断继续按原条件命题保留。规范NoPP、原义初始完成、父生命周期与完整F1/F2仍未建立。独评最终采用范围以其后续报告为准。

对应corrections-v1.json登记旧行号、字段、替代采用值、来源哈希、独评工件哈希及nested-author全目录前后指纹。没有改写仓库、正本、生产、formal或tracker。
