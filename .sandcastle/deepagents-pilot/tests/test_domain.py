from pilot1742.domain import (
    CommitIntent,
    CommitReceipt,
    EffectUnknown,
    HeadObservation,
    NotStarted,
    ReviewVerdict,
    RunFacts,
    Running,
    Terminal,
    WorkerKey,
    WorkerObservation,
    WorkerReceipt,
    classify_worker,
    current_head,
    derive_phase,
    dispatch_refusal,
    parse_sha,
    parse_verdict,
    reconcile_commit,
)

BASE, H1, H2 = (parse_sha(c * 40) for c in "abc")
TREE = "d" * 40


def receipt(role, head, status="completed", model="gpt-6.1-sol", summary="完成", changed=("demo/a.py",)):
    return WorkerReceipt(WorkerKey("r1", role, head), "child", status, model, head, head, changed, "s", "/log",
                         summary, (), (), None, "/out")


def obs(dir_exists=False, result=None, supervisor=False, group=False):
    return WorkerObservation(dir_exists, result, supervisor, group)


def test_classify_prefers_result_and_never_guesses_missing_effects():
    done = receipt("author", BASE)
    assert classify_worker(obs(True, done, supervisor=True)) == Terminal(done)
    assert classify_worker(obs(False, supervisor=True)) == Running()
    assert classify_worker(obs(False)) == NotStarted()
    assert isinstance(classify_worker(obs(True)), EffectUnknown)
    assert isinstance(classify_worker(obs(True, group=True)), EffectUnknown)


def test_commit_recognized_only_when_parent_tree_and_trailer_all_match():
    intent = CommitIntent(WorkerKey("r1", "author", BASE), BASE, TREE, ("demo/a.py",))
    assert reconcile_commit(intent, HeadObservation(BASE, None, "e" * 40, ())) == "not_committed"
    assert reconcile_commit(intent, HeadObservation(H1, BASE, TREE, (intent.trailer,))) == "committed"
    assert reconcile_commit(intent, HeadObservation(H1, BASE, "f" * 40, (intent.trailer,))) == "conflict"
    assert reconcile_commit(intent, HeadObservation(H1, BASE, TREE, ("r2/author-x",))) == "conflict"
    assert reconcile_commit(intent, HeadObservation(H2, H1, TREE, (intent.trailer,))) == "conflict"


def test_verdict_needs_protocol_line_and_cross_family():
    assert parse_verdict(receipt("review", H1, model="claude-opus-5-5", summary="VERDICT: PASS\n理由"), "gpt-6.1-sol").kind == "PASS"
    assert parse_verdict(receipt("review", H1, model="claude-opus-5-5", summary="VERDICT: BLOCK"), "gpt-6.1-sol").kind == "BLOCK"
    assert parse_verdict(receipt("review", H1, model="claude-opus-5-5", summary="看起来没问题"), "gpt-6.1-sol").kind == "UNPARSED"
    assert parse_verdict(receipt("review", H1, model="gpt-6.1-sol", summary="VERDICT: PASS"), "gpt-6.1-sol").kind == "VERIFIER_BLOCKED"
    assert parse_verdict(receipt("review", H1, "failed", "claude-opus-5-5", "VERDICT: PASS"), "gpt-6.1-sol").kind == "UNPARSED"


def facts(commits=(), workers=None, verdicts=None):
    return RunFacts("r1", BASE, tuple(commits), workers or {}, verdicts or {})


def commit(parent, head, author_head):
    return CommitReceipt(CommitIntent(WorkerKey("r1", "author", author_head), parent, TREE, ()), head, False)


def test_author_only_on_base_or_after_explicit_block():
    author_key = WorkerKey("r1", "author", H1)
    one_commit = [commit(BASE, H1, BASE)]
    done = {WorkerKey("r1", "author", BASE): Terminal(receipt("author", BASE))}
    assert dispatch_refusal(facts(), WorkerKey("r1", "author", BASE)) is None
    assert dispatch_refusal(facts(one_commit, done), author_key) is not None
    passed = {H1: ReviewVerdict(H1, "PASS", "gpt-6.1-sol", "claude-opus-5-5")}
    assert dispatch_refusal(facts(one_commit, done, passed), author_key) is not None
    blocked = {H1: ReviewVerdict(H1, "BLOCK", "gpt-6.1-sol", "claude-opus-5-5")}
    assert dispatch_refusal(facts(one_commit, done, blocked), author_key) is None


def test_review_needs_candidate_and_nothing_runs_twice():
    assert dispatch_refusal(facts(), WorkerKey("r1", "review", BASE)) is not None
    running = {WorkerKey("r1", "author", BASE): Running()}
    assert dispatch_refusal(facts([commit(BASE, H1, BASE)], running), WorkerKey("r1", "review", H1)) is not None
    unknown = {WorkerKey("r1", "author", BASE): EffectUnknown("x")}
    assert dispatch_refusal(facts([commit(BASE, H1, BASE)], unknown), WorkerKey("r1", "review", H1)) is not None


def test_head_follows_commit_chain_and_phase_tracks_facts():
    chain = [commit(H1, H2, H1), commit(BASE, H1, BASE)]
    assert current_head(facts(chain)) == H2
    assert derive_phase(facts()) == "needs_author"
    author_base = WorkerKey("r1", "author", BASE)
    assert derive_phase(facts(workers={author_base: Terminal(receipt("author", BASE))})) == "needs_commit"
    assert derive_phase(facts(workers={author_base: Terminal(receipt("author", BASE, changed=()))})) == "halted"
    assert derive_phase(facts([commit(BASE, H1, BASE)])) == "needs_review"
    blocked = {H1: ReviewVerdict(H1, "BLOCK", "gpt-6.1-sol", "claude-opus-5-5")}
    review = {WorkerKey("r1", "review", H1): Terminal(receipt("review", H1, model="claude-opus-5-5"))}
    assert derive_phase(facts([commit(BASE, H1, BASE)], review, blocked)) == "rework"
