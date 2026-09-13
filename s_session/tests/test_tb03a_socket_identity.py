"""#1374：各经济权威回复必须绑定明确的域、会话、代际及原因果请求。"""

import copy
import importlib.util
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location(
    "tb03a_socket_client", Path(__file__).resolve().parents[1] / "s_socket_client.py")
CLIENT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(CLIENT)


class EconomicReplyIdentityTests(unittest.TestCase):
    def request(self):
        payload = {"op": "ReadView", "cut": None}
        return {
            "schema_revision": "economic-session/1", "session_id": "session-7c",
            "session_generation": "3", "source_namespace": "query-source", "source_epoch": "8",
            "message_id": "query-1", "producer_id": "reader", "producer_epoch": "8",
            "payload": payload, "payload_hash": CLIENT.digest(payload), "causal_refs": [],
        }

    def reply(self, request, producer):
        payload = {"kind": "EconomicView", "actual_units": "9007199254740993", "stage": None}
        ph = CLIENT.digest(payload)
        return {
            **request, "producer_id": producer + ":session-7c", "producer_epoch": "11",
            "source_namespace": "economic-session/replies", "source_epoch": "11",
            "message_id": "reply-" + CLIENT.digest([
                request["source_namespace"], request["source_epoch"], request["message_id"], ph]),
            "payload": payload, "payload_hash": ph,
            "causal_refs": [{k: request[k] for k in (
                "source_namespace", "source_epoch", "message_id", "payload_hash")}],
        }

    def test_e_and_each_opaque_book_are_distinct_authorities(self):
        request = self.request()
        for owner in ("E", "B:7f31", "B:c284"):
            reply = self.reply(request, owner)
            result = CLIENT.validate_reply(request, reply, producer=owner, producer_epoch="11")
            self.assertEqual(result["actual_units"], "9007199254740993")
            for other in {"E", "B:7f31", "B:c284"} - {owner}:
                with self.subTest(owner=owner, other=other), self.assertRaises(ValueError):
                    CLIENT.validate_reply(request, reply, producer=other, producer_epoch="11")

    def test_valid_content_hash_cannot_substitute_domain_or_session(self):
        request = self.request()
        original = self.reply(request, "B:7f31")
        for key, replacement in (
            ("session_id", "session-other"), ("session_generation", "4"),
            ("producer_epoch", "10"), ("source_epoch", "10"),
            ("source_namespace", "s-session/replies"), ("causal_refs", []),
        ):
            modified = copy.deepcopy(original)
            modified[key] = replacement
            with self.subTest(key=key), self.assertRaises(ValueError):
                CLIENT.validate_reply(request, modified, producer="B:7f31", producer_epoch="11")

    def test_structure_schema_cannot_claim_economic_owner(self):
        request = self.request()
        request["schema_revision"] = "s-session/2"
        with self.assertRaises(ValueError):
            CLIENT.validate_reply(request, self.reply(request, "E"), producer="E", producer_epoch="11")


if __name__ == "__main__":
    unittest.main()
