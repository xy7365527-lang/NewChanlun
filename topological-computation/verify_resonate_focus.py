"""手动验证：resonate() 对话焦点集拓扑控制.

验证场景：
  1. 初始状态空
  2. resonate(["A", "B"]) → 焦点集={A,B}, active={A,B}
  3. resonate(["C"]) 且 C-A 有组合轴连接但 C-B 没有
     → 焦点集={A,C}(B退出焦点), active 中 B 如果和 A 或 C 无连接也退出
  4. 对话转向：resonate(["D"]) 且 D 和 A/C 都没有组合轴连接
     → 焦点集={D}, active 大幅收缩
"""

from signifier_net import SNet, Signifier, SignifierEdge, AxisType
from snet_activation import SNetActivation

# --- 构建测试 S_net ---
snet = SNet()
for sid in "ABCDE":
    snet = snet.add_signifier(Signifier(id=sid))

# 组合轴连接拓扑：
#   A -- B  (有边)
#   A -- C  (有边)
#   C -- E  (有边，E 是 C 的邻居但不在焦点集)
#   D 孤立（和 A/B/C/E 都没有组合轴连接）
snet = snet.add_edge(SignifierEdge(source="A", target="B", axis=AxisType.SYNTAGMATIC, weight=1.0))
snet = snet.add_edge(SignifierEdge(source="A", target="C", axis=AxisType.SYNTAGMATIC, weight=1.0))
snet = snet.add_edge(SignifierEdge(source="C", target="E", axis=AxisType.SYNTAGMATIC, weight=1.0))

# 构建 SNetActivation（空映射——resonate 不依赖 concept 映射）
activation = SNetActivation(snet, {}, {})

# --- 场景1：初始状态空 ---
assert activation.currently_active == set(), "初始 active 应为空"
assert activation._dialogue_focus_set == set(), "初始焦点集应为空"
print("PASS 场景1：初始状态空")

# --- 场景2：resonate(["A", "B"]) ---
activation.resonate(["A", "B"])
assert activation._dialogue_focus_set == {"A", "B"}, \
    f"焦点集应为 {{A,B}}，实际 {activation._dialogue_focus_set}"
assert activation.currently_active == {"A", "B"}, \
    f"active 应为 {{A,B}}，实际 {activation.currently_active}"
print("PASS 场景2：resonate(['A','B']) → 焦点集={A,B}, active={A,B}")

# --- 场景3：resonate(["C"]) ---
# C 和 A 有组合轴连接（A-C 边），C 和 B 没有
# 焦点集清理：
#   A 是旧焦点，A-C 有边 → A 留在焦点集
#   B 是旧焦点，B-C 没有边 → B 退出焦点集
# 焦点集 = {A, C}
# currently_active 清理：
#   B 不在焦点集，B 和焦点集成员（A, C）的连接？
#     B-A 有边 → B 留在 active
activation.resonate(["C"])
assert activation._dialogue_focus_set == {"A", "C"}, \
    f"焦点集应为 {{A,C}}，实际 {activation._dialogue_focus_set}"
# B 和 A 有组合轴连接（A-B 边），所以 B 留在 active
assert "B" in activation.currently_active, \
    f"B 和焦点集成员 A 有边，应留在 active，实际 {activation.currently_active}"
assert activation.currently_active == {"A", "B", "C"}, \
    f"active 应为 {{A,B,C}}，实际 {activation.currently_active}"
print("PASS 场景3：resonate(['C']) → 焦点集={A,C}, active={A,B,C} (B 通过 A 保留)")

# --- 场景4：对话转向 resonate(["D"]) ---
# D 和 A/C 都没有组合轴连接
# 焦点集清理：
#   A 是旧焦点，A-D 没有边 → A 退出焦点集
#   C 是旧焦点，C-D 没有边 → C 退出焦点集
# 焦点集 = {D}
# currently_active 清理：
#   A 不在焦点集，A-D 没有边 → A 退出
#   B 不在焦点集，B-D 没有边 → B 退出
#   C 不在焦点集，C-D 没有边 → C 退出
# active = {D}
activation.resonate(["D"])
assert activation._dialogue_focus_set == {"D"}, \
    f"焦点集应为 {{D}}，实际 {activation._dialogue_focus_set}"
assert activation.currently_active == {"D"}, \
    f"active 应为 {{D}}，实际 {activation.currently_active}"
print("PASS 场景4：对话转向 resonate(['D']) → 焦点集={D}, active={D} (大幅收缩)")

# --- 额外场景5：activate() 不影响焦点集 ---
# activate() 是穿越引擎的焦点切换，不操作 _dialogue_focus_set
activation2 = SNetActivation(snet, {"v1": "A", "v2": "B"}, {"A": ["v1"], "B": ["v2"]})
activation2.resonate(["A", "B"])
assert activation2._dialogue_focus_set == {"A", "B"}
activation2.activate("v1")  # activate A
# activate 只保留和新能指有组合轴连接的旧 active，不改焦点集
assert activation2._dialogue_focus_set == {"A", "B"}, \
    f"activate() 不应修改焦点集，实际 {activation2._dialogue_focus_set}"
print("PASS 场景5：activate() 不影响对话焦点集")

# --- 场景6：序列化/反序列化保留焦点集 ---
activation3 = SNetActivation(snet, {}, {})
activation3.resonate(["A", "C"])
d = activation3.to_dict()
assert "dialogue_focus_set" in d, "序列化应包含 dialogue_focus_set"
assert set(d["dialogue_focus_set"]) == {"A", "C"}

activation4 = SNetActivation(snet, {}, {})
activation4.restore_from_dict(d)
assert activation4._dialogue_focus_set == {"A", "C"}, \
    f"反序列化后焦点集应为 {{A,C}}，实际 {activation4._dialogue_focus_set}"
assert activation4.currently_active == {"A", "C"}, \
    f"反序列化后 active 应为 {{A,C}}，实际 {activation4.currently_active}"
print("PASS 场景6：序列化/反序列化保留焦点集")

print("\n全部验证通过。")
