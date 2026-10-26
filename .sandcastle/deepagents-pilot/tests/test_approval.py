import json

import pytest

from pilot1742.approval import (
    ApprovalObject,
    ApprovalRef,
    ApprovalRejected,
    ApprovalValid,
    AwaitingHuman,
    ClosedVerifier,
    ExclusiveAuthorVerifier,
    SpecTarget,
    judge_spec_authorization,
    load_approvers,
)
from pilot1742.delivery import Unpublishable, commit_payload

TARGET = SpecTarget(1340, "rev-7")
REF = ApprovalRef(4242, TARGET, "c-2")


def obj(object_id, order, *, author="alice", kind="User", decision="approve", ticket=4242, scope="spec_authorization",
        target=TARGET):
    return ApprovalObject(object_id, author, kind, ticket, scope, target, decision, order)


def judge(objects, approvers=frozenset({"alice"}), ref=REF, current=TARGET):
    return judge_spec_authorization(ref, ticket=4242, current_target=current, objects=objects, approvers=approvers)


def test_empty_exclusive_set_always_waits_for_a_human():
    assert judge([obj("c-2", 2)], approvers=frozenset()) == AwaitingHuman(("human_exclusive_approvers_empty",))
    assert isinstance(ClosedVerifier().verify(REF, ticket=4242, current_target=TARGET), AwaitingHuman)


def test_member_in_the_set_releases_exactly_the_referenced_latest_object():
    assert judge([obj("c-1", 1, decision="reject"), obj("c-2", 2)]) == ApprovalValid("c-2")
    assert ExclusiveAuthorVerifier((obj("c-2", 2),), frozenset({"alice"})).verify(
        REF, ticket=4242, current_target=TARGET) == ApprovalValid("c-2")


@pytest.mark.parametrize("objects, expected", [
    ([obj("c-2", 2, author="agent")], AwaitingHuman(("no_exclusive_approval",))),
    ([obj("c-2", 2, author="alice", kind="Bot")], AwaitingHuman(("no_exclusive_approval",))),
    ([obj("c-2", 2, ticket=9)], AwaitingHuman(("no_exclusive_approval",))),
    ([obj("c-2", 2, scope="main_merge")], AwaitingHuman(("no_exclusive_approval",))),
    ([obj("c-2", 2), obj("c-3", 3, decision="reject")], ApprovalRejected(("rejected",))),
    ([obj("c-2", 2), obj("c-3", 3)], ApprovalRejected(("ref_points_elsewhere",))),
    ([obj("c-2", 2, target=SpecTarget(1340, "rev-6"))], AwaitingHuman(("no_exclusive_approval",))),
])
def test_refusals(objects, expected):
    assert judge(objects) == expected


def test_changed_target_and_cross_ticket_refs_do_not_release():
    assert judge([obj("c-2", 2)], current=SpecTarget(1340, "rev-8")) == AwaitingHuman(("target_changed",))
    other = ApprovalRef(9, TARGET, "c-2")
    assert judge([obj("c-2", 2)], ref=other) == ApprovalRejected(("ref_for_other_ticket_or_scope",))


def test_approver_config_defaults_empty_and_refuses_bots(tmp_path):
    path = tmp_path / "registrar.json"
    assert load_approvers(path) == frozenset()
    path.write_text(json.dumps({"human_exclusive_approvers": ["newchanlun-deepagents-1742[bot]"]}))
    with pytest.raises(ValueError, match="bot"):
        load_approvers(path)


def test_signed_or_encoded_commits_are_unpublishable_before_any_ref():
    head = b"tree " + b"a" * 40 + b"\nauthor A <a@x> 1 +0000\ncommitter A <a@x> 1 +0000\n"
    assert commit_payload(head + b"\nmsg\n")["author"]["date"] == "1970-01-01T00:00:01Z"
    with pytest.raises(Unpublishable, match="gpgsig"):
        commit_payload(head + b"gpgsig -----BEGIN-----\n \n -----END-----\n\nmsg\n")
    with pytest.raises(Unpublishable, match="encoding"):
        commit_payload(head + b"encoding GBK\n\nmsg\n")
