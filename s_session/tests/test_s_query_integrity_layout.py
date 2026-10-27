"""#1491 ADR 0037：六数组次序按库 rule_revision 的布局组件只读重建；具名夹具，不代替真实 S/Q 验收。"""
import unittest

import s_query_integrity as graph

DOCTRINE = "s2-axis-quantifiers+new-bi/2"


def _shape(object_id, window_start, first, withdrawn=None):
    return {"object_id": object_id, "kind": "shape", "fact_key": "", "window_start": str(window_start),
            "first_known_generation": str(first),
            "withdrawn_generation": None if withdrawn is None else str(withdrawn)}


def _fixture():
    active = [_shape("o-late", 1, 3), _shape("o-a", 5, 1), _shape("o-b", 2, 2)]
    withdrawn = [_shape("o-w1", 9, 1, withdrawn=3), _shape("o-w2", 7, 2, withdrawn=2)]
    wires = {
        "witnesses": [(2, {"object_id": "o-a", "slot": "0"}), (1, {"object_id": "o-b", "slot": "1"}),
                      (1, {"object_id": "o-c", "slot": "0"}), (4, {"object_id": "o-0", "slot": "0"})],
        "relations": [(3, {"subject": "a", "relation_type": "r", "object": "x"}),
                      (1, {"subject": "b", "relation_type": "r", "object": "y"})],
        "observations": [(2, {"kind": "k", "window_start": None}), (1, {"kind": "k", "window_start": "4"}),
                         (1, {"kind": "j", "window_start": "9"})],
    }
    return active, withdrawn, wires


def _project(rule_revision, gen=3):
    active, withdrawn, wires = _fixture()
    layout = graph._rule_layout(rule_revision)[1]
    snapshot = graph.order_public_arrays({}, active, withdrawn, wires, gen, layout)
    return {
        "objects": [o["object_id"] for o in snapshot["objects"]],
        "withdrawn_objects": [o["object_id"] for o in snapshot["withdrawn_objects"]],
        "witnesses": [w["object_id"] for w in snapshot["witnesses"]],
        "relations": [r["subject"] for r in snapshot["relations"]],
        "observations": [(o["kind"], o["window_start"]) for o in snapshot["observations"]],
    }


class LayoutRuleTests(unittest.TestCase):
    def test_layout_component_split_matches_rust_split(self):
        self.assertEqual(graph._rule_layout(DOCTRINE), (DOCTRINE, graph.LAYOUT_SORTED))
        self.assertEqual(graph._rule_layout(DOCTRINE + "+bounded-layout/1"), (DOCTRINE, graph.LAYOUT_BOUNDED1))
        self.assertEqual(graph._published_axes(DOCTRINE + "+bounded-layout/1"), graph._published_axes(DOCTRINE))

    def test_layout_unknown_or_duplicate_component_is_rejected(self):
        for rule in (DOCTRINE + "+bounded-layout/2", DOCTRINE + "+bounded-layout/1+bounded-layout/1"):
            with self.assertRaisesRegex(ValueError, "VersionUnsupported"):
                graph._rule_layout(rule)
            with self.assertRaisesRegex(ValueError, "VersionUnsupported"):
                graph._published_axes(rule)


class LayoutOrderTests(unittest.TestCase):
    def test_layout1_keeps_original_content_key_order(self):
        self.assertEqual(_project(DOCTRINE), {
            "objects": ["o-late", "o-b", "o-a"],
            "withdrawn_objects": ["o-w1", "o-w2"],
            "witnesses": ["o-a", "o-b", "o-c"],
            "relations": ["a", "b"],
            "observations": [("j", "9"), ("k", None), ("k", "4")],
        })

    def test_bounded_layout_is_birth_order_then_original_key(self):
        self.assertEqual(_project(DOCTRINE + "+bounded-layout/1"), {
            "objects": ["o-a", "o-b", "o-late"],
            "withdrawn_objects": ["o-w2", "o-w1"],
            "witnesses": ["o-b", "o-c", "o-a"],
            "relations": ["b", "a"],
            "observations": [("j", "9"), ("k", "4"), ("k", None)],
        })

    def test_bounded_library_rejects_content_key_order(self):
        # 负控：bounded 库若按内容键（layout/1 次序）给出累计数组，与按布局重建的次序不符即拒收。
        bounded = _project(DOCTRINE + "+bounded-layout/1")
        content = _project(DOCTRINE)
        mismatched = [name for name in bounded if bounded[name] != content[name]]
        self.assertEqual(mismatched, ["objects", "withdrawn_objects", "witnesses", "relations", "observations"])


if __name__ == "__main__":
    unittest.main()
