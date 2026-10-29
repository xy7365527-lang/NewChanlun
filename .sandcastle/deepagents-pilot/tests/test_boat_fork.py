import io
import json
import time
from dataclasses import replace
from urllib.error import HTTPError, URLError
from types import SimpleNamespace

import pytest
from test_boat import FakeBoat, prepared_engine
from pilot1742.boat_execution import BoatForkAPI, BoatExecution, ExecutionUnknown, ForkIntent


def test_fork_response_loss_before_rename_reopens_same_sandbox(env, monkeypatch):
    cli = FakeBoat(limits=[{'canStart': True, 'creditUsedSeconds': 0}], rename=[{}],
                   info={'state': 'running', 'hydrated': True, 'archiveAfter': '2099-01-01T00:00:00Z'},
                   exec={}, scp=[], detach=[{'commandId': 'cmd_only', 'processId': 7}])
    run, engine, key, provisioned = prepared_engine(env, monkeypatch, cli)
    seen = []
    created = {}

    def fork(intent):
        persisted = engine.load(key).fork_intent
        assert persisted == intent
        seen.append(intent)
        created.setdefault(intent.idempotency_key, 'bx_original')
        if len(seen) == 1:
            raise ExecutionUnknown('response lost before rename')
        return created[intent.idempotency_key]

    engine.fork_api = SimpleNamespace(fork=fork)
    with pytest.raises(ExecutionUnknown):
        engine.submit(key)
    binding = engine.load(key)
    assert binding.sandbox_id is None and binding.create_requested
    assert not any(call[0] == 'rename' for call in cli.calls)
    reopened = BoatExecution(run, cli, SimpleNamespace(fork=fork))
    monkeypatch.setattr(reopened, '_build', engine._build)
    monkeypatch.setattr(reopened, '_provision_key', lambda binding: provisioned.append(binding.sandbox_id))
    reopened.submit(key)
    assert reopened.load(key).submission == 'submitted'
    assert reopened.load(key).sandbox_id == 'bx_original'
    assert reopened.load(key).fork_intent == binding.fork_intent
    assert seen[0] == seen[1] and len(created) == 1 and provisioned == ['bx_original']
    assert sum(call[0] == 'detach' for call in cli.calls) == 1
    run.release()


def test_expired_intent_never_calls_fork_again(env, monkeypatch):
    run, engine, key, _ = prepared_engine(env, monkeypatch, FakeBoat())
    intent = ForkIntent('bx_base', 'persisted-key', '{}', time.time() - 24 * 3600)
    engine._store(key, replace(engine.load(key), create_requested=True, fork_intent=intent))
    engine.fork_api = SimpleNamespace(fork=lambda intent: pytest.fail('expired request was replayed'))
    with pytest.raises(ExecutionUnknown, match='保留期已过'):
        engine.submit(key)
    run.release()


def intent():
    return ForkIntent('bx_base', 'same-key', '{"noEnv":true}', time.time())


def test_official_api_retries_in_progress_with_identical_key_and_body(monkeypatch):
    from pilot1742 import boat_execution
    monkeypatch.setenv('BOAT_API_KEY', 'fixture-controller-only')
    monkeypatch.setattr(boat_execution.time, 'sleep', lambda _: None)
    requests = []

    def request(req, timeout):
        requests.append((req.full_url, req.data, req.headers.copy()))
        if len(requests) == 1:
            raise HTTPError(req.full_url, 409, 'conflict', {},
                            io.BytesIO(b'{"code":"idempotency_in_progress"}'))
        return io.BytesIO(b'{"id":"bx_original"}')

    monkeypatch.setattr(boat_execution, 'urlopen', request)
    assert BoatForkAPI().fork(intent()) == 'bx_original'
    assert requests[0] == requests[1]
    assert requests[0][2]['Idempotency-key'] == 'same-key'


@pytest.mark.parametrize('failure', ['mismatch', 'transport'])
def test_official_api_preserves_unknown_without_new_key(monkeypatch, failure):
    from pilot1742 import boat_execution
    monkeypatch.setenv('BOAT_API_KEY', 'fixture-controller-only')
    requests = []

    def request(req, timeout):
        requests.append(req)
        if failure == 'transport':
            raise URLError('connection lost')
        raise HTTPError(req.full_url, 409, 'conflict', {},
                        io.BytesIO(b'{"code":"idempotency_key_reused"}'))

    monkeypatch.setattr(boat_execution, 'urlopen', request)
    with pytest.raises(ExecutionUnknown):
        BoatForkAPI().fork(intent())
    assert len(requests) == 1
