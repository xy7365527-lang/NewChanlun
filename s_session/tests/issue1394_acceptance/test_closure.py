"""只测内容编码/闭包拒绝，不制造财务或正式 Voice。"""
from dataclasses import replace
import time
import unittest
import closure as c
from driver import Rejected


class ClosureTests(unittest.TestCase):
    def setUp(self):
        self.nodes = {}

    def node(self, tag, body=b""):
        node = c.Node(c.node_root(tag, body), tag, body)
        self.nodes[node.root] = node
        return c.root_bytes(node.root)

    def seq(self, leaves):
        if not leaves:
            return self.node(0x11)
        if len(leaves) == 1:
            return self.node(0x12, leaves[0])
        split = 1 << ((len(leaves) - 1).bit_length() - 1)
        return self.node(0x13, len(leaves).to_bytes(8, "big") + self.seq(leaves[:split]) + self.seq(leaves[split:]))

    def string(self, raw, tag=3):
        leaves = [self.node(0x10, raw[i:i+4096]) for i in range(0, len(raw), 4096)]
        return self.node(tag, len(raw).to_bytes(8, "big") + self.seq(leaves))

    def verify(self, root, kind, extent, wire=None, source=None, **kwargs):
        target = {"format": "s-content/1", "root": "sc1-" + root.hex(), "kind": kind, "extent": extent}
        budget = kwargs.pop("budget", c.Budget(10000, 200000, 20000, time.monotonic() + 3))
        return c.verify(target, sorted(self.nodes.values(), key=lambda n: n.root) if wire is None else wire,
                        self.nodes.__getitem__ if source is None else source, budget, **kwargs)

    def test_frozen_null_vector_and_nonempty_string_terminal_byte(self):
        root = self.node(0)
        self.assertEqual(root.hex(), "8373191693a3a2ee636963cdfcae8775f1ca22610c77f1c11804dd3406bc5806")
        self.verify(root, "null", None)
        self.nodes.clear()
        root = self.string(b"x" * 4097)
        self.assertEqual(self.verify(root, "string", "4097")["product_status"], "NOT_RUN")

    def test_missing_cached_leaf_and_extra_unreachable_rejected(self):
        root = self.string(b"abc")
        wire = sorted(self.nodes.values(), key=lambda n: n.root)
        with self.assertRaises(Rejected):
            self.verify(root, "string", "3", wire=wire[:-1])
        self.node(2)
        with self.assertRaises(Rejected):
            self.verify(root, "string", "3")

    def test_duplicate_reordered_and_corrupt_original(self):
        root = self.string(b"abc")
        wire = sorted(self.nodes.values(), key=lambda n: n.root)
        for bad in (wire + wire[-1:], list(reversed(wire))):
            with self.subTest(bad=bad), self.assertRaises(Rejected):
                self.verify(root, "string", "3", wire=bad)
        with self.assertRaises(Rejected):
            self.verify(root, "string", "3", source=lambda r: replace(self.nodes[r], body=b"changed"))

    def test_extent_kind_internal_node_and_short_nonterminal_rejected(self):
        root = self.string(b"x" * 4097)
        for kind, size in (("number", "4097"), ("string", "4096")):
            with self.assertRaises(Rejected):
                self.verify(root, kind, size)
        self.nodes.clear()
        seq = self.seq([self.node(0x10, b"x"), self.node(0x10, b"y" * 4096)])
        root = self.node(3, (4097).to_bytes(8, "big") + seq)
        with self.assertRaises(Rejected):
            self.verify(root, "string", "4097")
        with self.assertRaises(Rejected):
            self.verify(seq, "array", "2")

    def test_typed_object_key_order_and_duplicate(self):
        for keys in ((b"a", b"b"), (b"b", b"a"), (b"a", b"a")):
            self.nodes.clear()
            val = self.node(0)
            fields = [self.node(0x14, self.string(k) + val) for k in keys]
            root = self.node(6, (2).to_bytes(8, "big") + self.seq(fields))
            if keys == (b"a", b"b"):
                self.verify(root, "object", "2")
            else:
                with self.assertRaises(Rejected):
                    self.verify(root, "object", "2")

    def test_same_digest_different_context_not_exempt_json_text(self):
        inner = self.node(7, self.node(0))
        outer = self.node(7, inner)
        root = self.node(5, (2).to_bytes(8, "big") + self.seq([inner, outer]))
        with self.assertRaises(Rejected):
            self.verify(root, "array", "2")

    def test_number_utf8_and_template_rejects(self):
        for raw, tag in ((b"01", 4), (b"NaN", 4), (b"\xff", 3)):
            self.nodes.clear()
            root = self.string(raw, tag)
            with self.assertRaises((Rejected, UnicodeDecodeError)):
                self.verify(root, "number" if tag == 4 else "string", str(len(raw)))
        self.nodes.clear()
        root = self.node(8)
        with self.assertRaises(Rejected):
            self.verify(root, "template_batch", None)
        with self.assertRaises(Rejected):
            self.verify(root, "template_batch", None, allow_template=True)

    def test_budget_shared_failure_no_refund(self):
        root = self.string(b"abc")
        budget = c.Budget(100, 1, 100, time.monotonic() + 3)
        with self.assertRaises(Rejected):
            self.verify(root, "string", "3", budget=budget)
        self.assertGreater(budget.bytes_read, budget.max_bytes)
        with self.assertRaises(Rejected):
            self.verify(root, "string", "3", budget=budget)

    def test_deep_graph_iterative_walk_not_python_recursion(self):
        root = self.node(0)
        for _ in range(1100):
            root = self.node(5, (1).to_bytes(8, "big") + self.seq([root]))
        self.verify(root, "array", "1", budget=c.Budget(3000, 1000000, 10000, time.monotonic()+4))
