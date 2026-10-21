#!/usr/bin/env python3
"""#1406 页客户端纯替身锁；不创建 socket/HTTP/产品进程。"""
import base64
import copy
import hashlib
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import s_socket_client as client

REV = "economic-read-pages/1"
MANIFEST = "a" * 64
CUT = {"domain_id": "E", "commit_seq": "7", "root_hash": "b" * 64}
QUERY = {"op": "ReadView", "cut": None, "as_known_ns": None}


def owner_document(padding=""):
    return {"kind": "EconomicView", "domain_id": "E", "cut": CUT,
            "writer_epoch": "3", "commit_ns": "100", "image": {"raw": padding},
            "image_hash": client.digest({"raw": padding}), "runtime": {}}


class Fake:
    """Independent byte producer: no production assembler or actual transport."""
    def __init__(self, value=None, *, query=None, public=False, raw=None):
        self.now = 100.0
        self.query = copy.deepcopy(QUERY if query is None else query)
        self.value = copy.deepcopy(owner_document() if value is None else value)
        self.raw = client.canonical(self.value) if raw is None else raw
        self.calls = []
        self.mutate = lambda op, response: response
        self.delay = {}
        self.public = public
        self.epochs = {"E": "3"}
        kind = self.value["kind"]
        if public:
            fixed = {k: (v.get("cut" if kind == "EconomicSnapshot" else "next_cut")
                         if v["status"] == "Available" else None)
                     for k, v in self.value["domains"].items()}
        else:
            fixed = {"E": self.value["cut"]}
        self.descriptor = {"schema_revision": REV, "transfer_id": "c" * 32,
            "session_id": "session", "session_generation": "2", "manifest_hash": MANIFEST,
            "producer_id": ("Q:" if public else "E:") + "session", "producer_epoch": "3",
            "query_sha256": client.digest(self.query), "document_kind": kind,
            "fixed_cuts": fixed, "owner_epochs": self.epochs, "document_bytes": str(len(self.raw)),
            "document_sha256": hashlib.sha256(self.raw).hexdigest(), "chunk_bytes": "262144",
            "page_count": str((len(self.raw) + 262143) // 262144)}

    def clock(self):
        return self.now

    def fetch(self, request, remaining_ms):
        self.calls.append((copy.deepcopy(request), remaining_ms))
        op = request["op"]
        self.now += self.delay.get(op, 0)
        dh = client.digest(self.descriptor)
        if op == "EconomicReadOpen":
            response = {"schema_revision": REV, "kind": "EconomicReadOpened",
                        "descriptor": copy.deepcopy(self.descriptor), "descriptor_sha256": dh}
        elif op == "EconomicReadPage":
            start = int(request["offset"])
            raw = self.raw[start:start + 262144]
            end = start + len(raw)
            response = {"schema_revision": REV, "kind": "EconomicReadPage",
                "transfer_id": self.descriptor["transfer_id"], "descriptor_sha256": dh,
                "offset": str(start), "raw_byte_count": str(len(raw)),
                "chunk_sha256": hashlib.sha256(raw).hexdigest(),
                "data_base64": base64.b64encode(raw).decode(),
                "next_offset": None if end == len(self.raw) else str(end), "done": end == len(self.raw)}
        else:
            response = {"schema_revision": REV, "kind": "EconomicReadClosed",
                        "transfer_id": self.descriptor["transfer_id"], "descriptor_sha256": dh}
        return self.mutate(op, response)

    def read(self, **overrides):
        args = dict(session_id="session", session_generation="2",
            producer_id=("Q:" if self.public else "E:") + "session", producer_epoch="3",
            manifest_hash=MANIFEST, owner_epochs=self.epochs, deadline=130.0, clock=self.clock)
        args.update(overrides)
        return client.read_economic_pages(self.query, self.fetch, **args)


def snapshot(*, unavailable=False):
    domain = ({"status": "Unavailable", "error": "offline"} if unavailable else
              {"status": "Available", "cut": CUT, "commit_ns": "100", "image": {},
               "image_hash": client.digest({})})
    value = {"kind": "EconomicSnapshot", "session_id": "session", "session_generation": "2",
        "manifest_hash": MANIFEST, "mode": "Current", "as_known_ns": None,
        "domains": {"E": domain}, "completeness": "PartialVector" if unavailable else "CompleteVector"}
    value["snapshot_token"] = "economic-snapshot-" + client.digest(value)
    return value


class PageClientTests(unittest.TestCase):
    def test_four_document_kinds(self):
        view = Fake()
        self.assertEqual(view.read(), view.value)
        changes = owner_document()
        changes["kind"] = "EconomicChanges"
        changes["changes"] = []
        own_watch = Fake(changes, query={"op": "Watch", "after_cut": CUT})
        self.assertEqual(own_watch.read(), own_watch.value)
        snap = Fake(snapshot(), public=True, query={"op": "snapshot", "cuts": None,
                    "as_known_ns": None, "snapshot_token": None})
        self.assertEqual(snap.read(), snap.value)
        watch_value = {"kind": "EconomicWatch", "session_id": "session", "session_generation": "2",
            "manifest_hash": MANIFEST, "domains": {"E": {"status": "Available", "base_cut": CUT,
            "next_cut": CUT, "changes": []}}}
        watch = Fake(watch_value, public=True, query={"op": "watch", "cuts": {"E": CUT}})
        self.assertEqual(watch.read(), watch.value)
        for fake in (view, own_watch, snap, watch):
            self.assertEqual(fake.calls[-1][0]["op"], "EconomicReadClose")

    def test_partial_vector_remains_partial(self):
        fake = Fake(snapshot(unavailable=True), public=True,
                    query={"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None})
        self.assertEqual(fake.read()["completeness"], "PartialVector")

    def test_snapshot_token_reread(self):
        value = snapshot()
        fake = Fake(value, public=True, query={"op": "snapshot", "cuts": None,
                    "as_known_ns": None, "snapshot_token": value["snapshot_token"]})
        self.assertEqual(fake.read(), value)

    def test_large_single_record_exact_reconstruction_utf8_escape_boundaries(self):
        fake = Fake(owner_document(('中\\"\n' * 130000)))
        self.assertGreater(len(fake.raw), 1048576)
        self.assertEqual(client.canonical(fake.read()), fake.raw)
        pages = [x[0] for x in fake.calls if x[0]["op"] == "EconomicReadPage"]
        self.assertEqual([int(x["offset"]) for x in pages], list(range(0, len(fake.raw), 262144)))

    def test_exact_document_limit(self):
        empty = owner_document()
        room = 4194304 - len(client.canonical(empty))
        fake = Fake(owner_document("x" * room))
        self.assertEqual(len(fake.raw), 4194304)
        self.assertEqual(fake.read(), fake.value)

    def test_declared_over_limit_rejected_before_page(self):
        fake = Fake()
        fake.descriptor["document_bytes"] = "4194305"
        with self.assertRaises(ValueError):
            fake.read()
        self.assertFalse(any(x[0]["op"] == "EconomicReadPage" for x in fake.calls))
        self.assertEqual(fake.calls[-1][0]["op"], "EconomicReadClose")

    def test_descriptor_mutations(self):
        cases = {"schema_revision": "other", "transfer_id": "C" * 32, "session_id": "other",
            "session_generation": True, "manifest_hash": "d" * 64, "producer_id": "B:x:session",
            "producer_epoch": "4", "query_sha256": "e" * 64, "document_kind": "EconomicChanges",
            "owner_epochs": {"E": "4"}, "chunk_bytes": "262145", "page_count": "2",
            "fixed_cuts": {"B:x": CUT}, "document_bytes": True}
        for field, value in cases.items():
            with self.subTest(field=field):
                fake = Fake()
                fake.descriptor[field] = value
                with self.assertRaises(ValueError):
                    fake.read()

    def test_descriptor_exact_fields_and_hash(self):
        for missing in (True, False):
            fake = Fake()
            if missing:
                del fake.descriptor["document_bytes"]
            else:
                fake.descriptor["extra"] = "x"
            with self.assertRaises(ValueError):
                fake.read()
        fake = Fake()
        fake.mutate = lambda op, value: {**value, "descriptor_sha256": "f" * 64}
        with self.assertRaises(ValueError):
            fake.read()

    def test_page_mutations(self):
        cases = {"schema_revision": "other", "kind": "Other", "transfer_id": "0" * 32,
            "descriptor_sha256": "f" * 64, "offset": "1", "raw_byte_count": "0",
            "next_offset": "1", "done": 1, "data_base64": "!!!", "chunk_sha256": "f" * 64}
        for field, bad in cases.items():
            with self.subTest(field=field):
                fake = Fake()
                fake.mutate = lambda op, value: ({**value, field: bad} if op == "EconomicReadPage" else value)
                with self.assertRaises(ValueError):
                    fake.read()
                self.assertEqual(fake.calls[-1][0]["op"], "EconomicReadClose")

    def test_noncanonical_integers(self):
        for value in (True, False, 0, 1, "01", "-1", "+1", " 1", "1.0", "", "９"):
            for field in ("document_bytes", "page_count"):
                with self.subTest(value=value, field=field):
                    fake = Fake()
                    fake.descriptor[field] = value
                    with self.assertRaises(ValueError):
                        fake.read()

    def test_duplicate_skip_overlap_truncated_pages(self):
        for bad_offset in ("0", "262145", "262143"):
            fake = Fake(owner_document("x" * 600000))
            def corrupt(op, value):
                if op == "EconomicReadPage" and value["offset"] == "262144":
                    value["offset"] = bad_offset
                return value
            fake.mutate = corrupt
            with self.assertRaises(ValueError):
                fake.read()

    def test_document_tamper_and_strict_decode(self):
        raw_cases = [b'{"kind":"EconomicView","kind":"EconomicView"}',
                     b'{"x":1.0}', b'{"x":NaN}', b'{"x":true}tail', b'{"x":true}\n',
                     b'{"x": true}', b'\xff']
        for raw in raw_cases:
            with self.subTest(raw=raw):
                fake = Fake(raw=raw)
                with self.assertRaises((ValueError, UnicodeError)):
                    fake.read()
        fake = Fake()
        fake.descriptor["document_sha256"] = "f" * 64
        with self.assertRaises(ValueError):
            fake.read()

    def test_body_cannot_change_descriptor_cut(self):
        fake = Fake()
        fake.descriptor["fixed_cuts"]["E"] = {**CUT, "root_hash": "f" * 64}
        with self.assertRaises(ValueError):
            fake.read()

    def test_root_deadline_covers_open_page_close(self):
        for op in ("EconomicReadOpen", "EconomicReadPage", "EconomicReadClose"):
            with self.subTest(op=op):
                fake = Fake()
                fake.delay[op] = 30
                with self.assertRaises(TimeoutError):
                    fake.read()

    def test_many_pages_share_decreasing_remaining(self):
        fake = Fake(owner_document("x" * 600000))
        fake.delay = {"EconomicReadOpen": 2, "EconomicReadPage": 11}
        with self.assertRaises(TimeoutError):
            fake.read()
        left = [remaining for _, remaining in fake.calls]
        self.assertEqual(left, sorted(left, reverse=True))
        self.assertLess(left[-1], left[0])

    def test_earlier_parent_deadline_and_expired_no_send(self):
        fake = Fake()
        with self.assertRaises(TimeoutError):
            fake.read(deadline=100)
        self.assertEqual(fake.calls, [])
        fake.delay["EconomicReadPage"] = 6
        with self.assertRaises(TimeoutError):
            fake.read(deadline=105)

    def test_close_failure_cannot_deliver(self):
        fake = Fake()
        fake.mutate = lambda op, value: ({**value, "transfer_id": "0" * 32}
                                        if op == "EconomicReadClose" else value)
        with self.assertRaises(ValueError):
            fake.read()

    def test_error_frame_never_becomes_document(self):
        for stage in ("EconomicReadOpen", "EconomicReadPage", "EconomicReadClose"):
            fake = Fake()
            fake.mutate = lambda op, value: ({"schema_revision": REV, "kind": "EconomicReadError",
                "code": "TransferExpired", "detail": "expired", "transfer_id": "c" * 32}
                if op == stage else value)
            with self.assertRaises(ValueError):
                fake.read()

    def test_lower_caps_and_higher_caps(self):
        fake = Fake(owner_document("x" * 2000))
        for kw in ({"max_frame_bytes": 1048577}, {"max_document_bytes": 4194305},
                   {"max_frame_bytes": True}, {"max_document_bytes": 1000}, {"max_frame_bytes": 100}):
            with self.subTest(kw=kw), self.assertRaises(ValueError):
                fake.read(**kw)

    def test_image_hash_recomputed(self):
        value = owner_document()
        value["image_hash"] = "f" * 64
        with self.assertRaises(ValueError):
            Fake(value).read()
        value = snapshot()
        value["domains"]["E"]["image_hash"] = "f" * 64
        value["snapshot_token"] = "economic-snapshot-" + client.digest(
            {k: v for k, v in value.items() if k != "snapshot_token"})
        with self.assertRaises(ValueError):
            Fake(value, public=True, query={"op": "snapshot", "cuts": None,
                 "as_known_ns": None, "snapshot_token": None}).read()

    def test_watch_sequence_time_and_final_cut(self):
        base = {**CUT, "commit_seq": "5"}
        changes = [{"cut": {**CUT, "commit_seq": str(n)}, "image": {},
                    "image_hash": client.digest({}), "commit_ns": str(n * 10)} for n in (6, 7)]
        value = {"kind": "EconomicChanges", "domain_id": "E", "cut": CUT,
                 "writer_epoch": "3", "changes": changes}
        query = {"op": "Watch", "after_cut": base}
        self.assertEqual(Fake(value, query=query).read(), value)
        mutations = [lambda v: v["changes"].pop(),
                     lambda v: v["changes"][1].update(commit_ns="1"),
                     lambda v: v["changes"][0]["cut"].update(commit_seq="7"),
                     lambda v: v["changes"][0]["cut"].update(domain_id="B:x"),
                     lambda v: v["changes"][0].update(image_hash="f" * 64),
                     lambda v: v.update(changes=changes * 17)]
        for mutate in mutations:
            with self.subTest(mutate=mutate):
                bad = copy.deepcopy(value)
                mutate(bad)
                with self.assertRaises(ValueError):
                    Fake(bad, query=query).read()

    def test_watch_from_genesis(self):
        cut = {**CUT, "commit_seq": "0"}
        value = {"kind": "EconomicChanges", "domain_id": "E", "cut": cut,
                 "writer_epoch": "3", "changes": [{"cut": cut, "image": {},
                 "image_hash": client.digest({}), "commit_ns": "0"}]}
        self.assertEqual(Fake(value, query={"op": "Watch", "after_cut": None}).read(), value)
        value["changes"] = []
        with self.assertRaises(ValueError):
            Fake(value, query={"op": "Watch", "after_cut": None}).read()

    def test_snapshot_token_mode_and_partial_cut_mismatch(self):
        query = {"op": "snapshot", "cuts": None, "as_known_ns": None, "snapshot_token": None}
        for field, wrong in (("snapshot_token", "bad"), ("mode", "AsKnown"),
                             ("completeness", "PartialVector"), ("manifest_hash", "f" * 64)):
            with self.subTest(field=field):
                value = snapshot()
                value[field] = wrong
                with self.assertRaises(ValueError):
                    Fake(value, query=query, public=True).read()
        fake = Fake(snapshot(unavailable=True), query=query, public=True)
        fake.descriptor["fixed_cuts"]["E"] = CUT
        with self.assertRaises(ValueError):
            fake.read()

    def test_as_known_future_rejected(self):
        query = {"op": "ReadView", "cut": None, "as_known_ns": "99"}
        with self.assertRaises(ValueError):
            Fake(query=query).read()

    def test_page_exact_fields(self):
        for missing in (False, True):
            fake = Fake()
            def mutate(op, value):
                if op == "EconomicReadPage":
                    if missing:
                        del value["done"]
                    else:
                        value["extra"] = None
                return value
            fake.mutate = mutate
            with self.assertRaises(ValueError):
                fake.read()

    def test_noncanonical_base64_padding_bits(self):
        fake = Fake()
        while len(fake.raw) % 3 == 0:
            fake = Fake(owner_document("x"))
        def mutate(op, value):
            if op == "EconomicReadPage":
                alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
                encoded = value["data_base64"]
                n = len(encoded.rstrip("=")) - 1
                # Set an unused pad bit: decoding yields identical bytes, re-encoding does not.
                value["data_base64"] = encoded[:n] + alphabet[alphabet.index(encoded[n]) + 1] + encoded[n + 1:]
            return value
        fake.mutate = mutate
        with self.assertRaises(ValueError):
            fake.read()

    def test_repeated_nonfinal_page_not_done_and_no_empty_chunk(self):
        for field, bad in (("done", True), ("next_offset", None), ("raw_byte_count", "0")):
            fake = Fake(owner_document("x" * 300000))
            fake.mutate = lambda op, value: ({**value, field: bad} if op == "EconomicReadPage" else value)
            with self.assertRaises(ValueError):
                fake.read()


if __name__ == "__main__":
    unittest.main()
