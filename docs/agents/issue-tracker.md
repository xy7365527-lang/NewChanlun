# Issue tracker: GitHub

Issues and PRDs for this repo live as GitHub issues. Use the `gh` CLI for all operations.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

Infer the repo from `git remote -v` — `gh` does this automatically when run inside a clone.

## `main-rewritten` 与 CI

`main-rewritten` 是本地 `main` 的事实镜像，约每日推送一次；`origin/main` 是停在 2026-04-19 的死分支，已与本地 `main` 分叉 2602/1171 个提交。CI 触发器挂在 `main-rewritten`，因为它才是远端可触发且持续承载本地 `main` 事实的镜像分支（#457）。

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>` for the diff.
- **List external PRs for triage**: `gh pr list --state open --json number,title,body,labels,author,authorAssociation,comments` then keep only `authorAssociation` of `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, or `NONE` (drop `OWNER`/`MEMBER`/`COLLABORATOR`).
- **Comment / label / close**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either — resolve with `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --comments`.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `gh issue create --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (`gh api` on the sub-issues endpoint). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Title prefix must match the label**: the `[<type>]` title prefix and the `wayfinder:<type>` label must agree, and **both must be aligned to what the ticket actually does** — do not copy one side onto the other, or you propagate the wrong one. Machine behaviour (frontier queries, sub-issue counts) reads `--label` only; the prefix is for humans, and a mismatch misleads them. Self-enforced by declaration, no sweep. See [丁类/戊类票的归属 #782](https://github.com/xy7365527-lang/NewChanlun/issues/782).
- **Listing sub-issues — always `--paginate`**: `gh api repos/<owner>/<repo>/issues/<n>/sub_issues` returns **only 30 rows per page by default**. Maps with more than 30 children (this repo already has [#529](https://github.com/xy7365527-lang/NewChanlun/issues/529) = 43 and [#695](https://github.com/xy7365527-lang/NewChanlun/issues/695) = 34) **silently lose tickets** without it, and a `/30` count reads as a total. Always `gh api --paginate repos/<owner>/<repo>/issues/<n>/sub_issues`. Same for `gh issue list` — pass `--limit` well above the expected count.
- **Blocking**: GitHub's **native issue dependencies** — the canonical, UI-visible representation. Add an edge with `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). GitHub reports `issue_dependencies_summary.blocked_by` (open blockers only — the live gate). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list the map's open children (`gh api --paginate repos/<owner>/<repo>/issues/<map>/sub_issues` — **`--paginate` is mandatory**, see above — or `gh issue list --state open --limit 200` scoped to the map's task list), drop any with an open blocker (`issue_dependencies_summary.blocked_by > 0`, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Reconciliation** (run at the start of every wayfinding session): list the map's **closed** children (`gh api --paginate .../sub_issues`, filter `state == "closed"`) and diff against the rows already in the map's Decisions-so-far; add whatever is missing. Costs one query and caps index drift at one session. Write protocol for the map body is in `docs/agents/wayfinder-workflow.md`.
- **Claim**: `gh issue edit <n> --add-assignee @me` — the session's first write.
- **Resolve**: `gh issue comment <n> --body "<answer>"`, then `gh issue close <n>`, then append a context pointer (gist + link) to the map's Decisions-so-far.
