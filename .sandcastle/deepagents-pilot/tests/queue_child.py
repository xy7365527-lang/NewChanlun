import json
import sys
from multiprocessing.connection import Connection
from pathlib import Path
from unittest.mock import patch

import httpx
from approval_signing import authority, decision
from approval_signing.protocol import parse_trust
from conftest import FAKE, MODELS

from pilot1742.approval import SignedApprovals
from pilot1742.boat_execution import HistoricalLocal
from pilot1742.claims import ClaimStore
from pilot1742.cutover import CutoverStore
from pilot1742.delivery import RepoGateway
from pilot1742.executor import TicketExecutor, _sigkill_at, load_unit, open_v2_run
from pilot1742.github_app import WRITE_PERMISSIONS, AppIdentity, GitHubAppClient
from pilot1742.workunit import REPO

config = json.loads(Path(sys.argv[1]).read_text())
channel = Connection(int(sys.argv[2]))


def request(req):
    channel.send((req.method, str(req.url), dict(req.headers), req.content))
    status, headers, content = channel.recv()
    return httpx.Response(status, headers=headers, content=content)


decision.load_trust = lambda path: parse_trust(Path(config['trust']).read_bytes())
authority.trust_path_problem = lambda path: None
home = Path(config['home'])
unit, task = load_unit(home, config['run_id'])
identity = AppIdentity(5486877, 175617978, 1150335160, REPO, 'newchanlun-deepagents-1742', Path(config['pem']))
gateway = RepoGateway(GitHubAppClient(identity, transport=httpx.MockTransport(request), permissions=WRITE_PERMISSIONS))
executor = TicketExecutor(unit, gateway, claims=ClaimStore(Path(config['claims'])),
                          workers_log=Path(config['workers']),
                          approvals=SignedApprovals(authority_root=Path(config['authority'])),
                          cutover=CutoverStore(home, Path(config['drain']), Path(config['workers'])),
                          crash=_sigkill_at(config['crash']))
with patch('pilot1742.run.new_execution', return_value=HistoricalLocal('test-launcher')):
    run = open_v2_run(home, unit, task, executor, MODELS, launcher=FAKE, source_repo=Path(config['source']))
try:
    if not run.facts().commits:
        run.run_worker('author', '实现')
        run.commit_candidate()
        run.run_worker('review', '审')
    assert run.publish_candidate()['phase'] == 'awaiting_merge'
finally:
    run.release()
    channel.close()
