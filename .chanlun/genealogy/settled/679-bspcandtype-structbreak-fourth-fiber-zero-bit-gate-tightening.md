---
id: "679"
number: 679
type: concept-separation   # 与 673 同族——统一分支内部混入几何前提互斥的对象群，正确解法=拆分谓词/分支而非下游打补丁；本号是 673 修复（Type1/2/3 三分拆）之后暴露的第四类残余互斥（else=>Type3 仍吞并零 bit 破中枢候选）。
status: 已结算   # codex 终局裁决A（`.chanlun/review-results/codex-decide-20260702-204130-d07f.md`）给出唯一施工级规格，已逐字实装+护栏测试+350K BTC dx 重测，无待裁决分叉。
settled_date: "2026-07-02"
settled_by: "ws-sbimpl（task #62，team-lead 指派）"
date: "2026-07-02"
source: "codex 终局裁决A `.chanlun/review-results/codex-decide-20260702-204130-d07f.md`（mode=decide，三选一 A/B/C，选 A：新增 BspCandType::StructBreak 第四类纤维）"
negation_source: heterogeneous
negation_model: "codex-cli，裁决：T3 纤维语义不可挽救——`bsp_cand_type` else 分支无差别吞并两个几何前提互斥的对象群（真三类候选 vs P2-R2 零 bit 破中枢未背驰候选），673 号先例要求拆分而非合并"
negation_form: separation

topo_effect: "sever:bsp-cand-type-else-branch:structbreak-fiber"
# 673 已把 Type1/Type2/Type3 三分拆为独立函数+dispatcher，但 Type3 判别条件仍是「else（¬buy1∧¬buy2∧¬buy3/sell 同构）」——这个 else 分支本身仍是一个未分拆的混合陪域，混入了(i)真三类候选（未破核心区间回试，buy3/sell3 置位）和(ii) P2-R2 零 bit 破中枢未背驰候选（class_index()==0，方向靠 struct_break_dir 旁路恢复，不进 bits）。本号 sever 把(ii)从 else 分支中切出为独立第四类 StructBreak。

depends_on:
  - "673"   # 直接先例——「统一谓词/分支混入互斥语义须拆分」的结算原则，本号是该原则在 else=>Type3 分支上的第二次应用（673 处理 Type1 vs Type2 互斥，本号处理 Type3 vs StructBreak 互斥）。
  - "615"   # partition-proof-20260702.md §9.2 首次逐行核实此互斥（T3 纤维语义不可挽救的证据链）。

# 涉及的定义
definitions_involved:
  - name: "P2-R2（破中枢结构候选全部进样本，消选择偏差）"
    version: "signal.rs:577-580 注释（codex-decide-20260701-2121 → p2-plan §2）"
    role: "零 bit 破中枢未背驰候选的产生机制——趋势∧破最后中枢∧A/C可配对全部进样本，未背驰确认时 bits 全零 + struct_break_dir 方向旁路。"
  - name: "673-fix（Cand^δ_ℓ 接口级三分拆，已结算）"
    version: ".chanlun/genealogy/settled/673-cand-delta-predicate-scope-misuse-type1-divergence-vs-type2-completeness-h2-overfiltering.md"
    role: "上游先例——本号是 673 结算原则（互斥语义混入同一分支须拆分）在 else=>Type3 分支上的延伸应用，非重复发现。"

proposition:
  claim: "BspCandType::Type3 的 else 分支（¬buy1∧¬buy2 判别）无差别吞并两个几何前提互斥的对象群，其中零 bit 破中枢未背驰候选（class_index()==0）应独立为第四类 StructBreak，门控层恒拒（无对应确认语义），不复用 Type3 的 Nest/Xzd 通道。"
  mechanism: "群体(i) 真三类候选（judge_third 产生，signal.rs:358-391）几何前提=未破核心区间（retest.price>c.zg 严格不触及），置 buy3/sell3。群体(ii) P2-R2 零 bit 候选几何前提=已破最后中枢（P2-R2 入样本判据字面「破最后中枢」），bits 全零、方向靠 struct_break_dir 旁路（candidate_dir 仅当六 bit 严格全零时启用该旁路，interp.rs:220-236 + 护栏测试 interp.rs:1059-1074）。两前提不可能同时成立，`else => Type3` 无条件合并二者。修复：`bsp_cand_type` 前置 `bits.class_index()==0` 判别 ⟹ StructBreak，cand_delta/cand_delta_base_gate/build_gate_certificate 三处 match 新增 `StructBreak => false/false/None`（门拒，不新增 GateCertificate 通道——Nest/Xzd 是确认类证书通道，零 bit 破中枢未背驰候选无对应确认语义，现在新增第三通道是过度设计）。"
  epistemic_level: "L0（纯结构分类改动，确定性——StructBreak 判别条件 `class_index()==0` 与既有护栏测试 struct_break_dir_recovers_direction_without_touching_class_index 的断言完全对齐，非新假设）+ L2（350K BTC 真实数据重测 gate-pass 信号集变化量，见下）。"

classification:
  contradiction: "定理性发现——673 已结算的原则（统一谓词/分支混入互斥语义须拆分）在 else=>Type3 分支上仍有未处理的残余实例，性质与 673 相同（几何前提互斥的对象群被单一分支无差别吞并）。"
  fix: "codex 全权终局裁决，唯一方案（A），非选择类——已按施工级规格逐字实装。"

tension_check:
  family: "673（Type1 Extreme vs Type2 完备性互斥）→ 679（本号，Type3 未破核心区间 vs StructBreak 已破中枢互斥）——同一模式（统一分支吞并互斥几何前提）在候选分类的不同分支上的第二次实例。净新维度=本号的互斥对象不是 Type1/Type2/Type3 之间的判据混淆，而是「有明确原文对象定义的三类买卖点」vs「工程性消偏差样本保留的零 bit 结构候选」——后者在原文中没有对应地位，是纯粹的样本完备性设计（P2-R2），与三类买卖点的原文定义不同源。故不与 673 背驰，而是巩固「统一分支须拆分」的结算原则。"
  vs_673: "无矛盾——673 处理 else 分支之前（Type1 vs Type2 判据混淆），本号处理 else 分支内部残留的第四类混入（Type3 vs StructBreak）。两者是同一 codex 终局裁决①（本次 decide）识别的连续两层问题，非重复。"
  new_pending_genealogy: "无新矛盾涌现——本号是 673 结算原则的第二次实例应用，收口，非新族。"

# 350K BTC dx 重测（task #62 交办，如实报告）
empirical_measurement:
  method: "acc_classification_level_hole_dx（ECON_L2_MAX_BARS=350000，真实 BTC 数据，--release --ignored --nocapture）。新增计数器 n_zerobit_gate_pass：统计 bsp_class==0（=class_index()==0）候选中通过 build_gate_certificate 门的条数。"
  before: "临时禁用 StructBreak 前置判别（还原旧 else=>Type3 语义），350K 窗测得 n_zerobit_gate_pass=0（n_gate_pass_total=892, n_xzd_pass=30，sig_post_sum=892）。"
  after: "恢复 StructBreak 判别，同窗重测 n_zerobit_gate_pass=0（n_gate_pass_total=892, n_xzd_pass=30，sig_post_sum=892——与 before 完全一致）。"
  delta: "0——本次收紧在 350K BTC 实测窗对已产出的 gate-pass 信号集**无经验影响**（该窗内零 bit 破中枢候选此前也从未真正走通 Nest/Xzd 门通过）。收紧的价值是**架构性/概念性**（消除 T3 纤维语义的对象混淆，closes 673 先例的残余实例），不是本窗口的信号集收窄。如实报告：即使为 0 也不隐藏（formalization-validity-domain 规则，L2 真实数据可产否定性结果——本次即否定性结果：收紧无本窗实测效应）。"
  caveat: "有效域=350K 窗（本仓库全量 461 万 bar 的截断子窗，non-full-history claim）。不同窗口/不同标的可能出现非零 delta，未来 W-VERIFY（task #13）全量重测时应一并核对本计数器是否仍恒 0。"

impact:
  - "改动模块：rust/src/theta_v0/backtest/econ_positive.rs——BspCandType 枚举（+StructBreak）、bsp_cand_type（前置 class_index()==0 判别）、cand_delta/cand_delta_base_gate/build_gate_certificate 三处 match（各 +StructBreak 分支）。"
  - "新增单测护栏：bsp_cand_type_structbreak_zero_bits_dispatch（StructBreak 分派 + buy3/sell3 真三类不受影响）、build_gate_certificate_structbreak_zero_bits_always_none（零 bit 恒 None + lvl==0 免门通道不误纳 + 真 Type3 对照组仍 Some）。"
  - "诊断新增：acc_classification_level_hole_dx 新增 n_zerobit_gate_pass 计数器 + 报告行，供未来重测复查。"
  - "不改任何 settled 定理——本号是 673 已结算原则的第二次实例应用（延伸，非否定）。"
  - "测试验证：cargo test --release econ_positive（33 passed, 0 failed, 7 ignored）+ cargo test --release --lib（1396 passed, 0 failed, 100 ignored）全绿。"
  - "下游：task #13（W-VERIFY alpha 全量重测）应在本次收紧后重跑——alpha 分桶键 MuClass.i_class 不受影响（codex 裁决已订正：i_class=bits.class_index()，非 BspCandType，零 bit 候选在分桶层本就识别为 bsp_class=0，与真三类不混），本次收紧只影响 econ_positive.rs 内部门控分派（Nest/Xzd/拒），不影响 W-VERIFY 桶键完备性。"
