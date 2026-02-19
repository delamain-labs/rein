# Development Process

## Workflow
1. **Ticket** — Every piece of work starts as a GitHub issue
2. **Branch** — Create a branch from `master` for each issue
3. **Build (TDD)** — Write tests first, then implementation.
4. **PR** — Push branch, open PR referencing the issue
5. **Review** — Sub-agent reviewer provides feedback on the PR
6. **Address** — Fix review feedback, push updates
7. **Approve + Merge** — Reviewer confirms, PR gets merged
8. **Out-of-scope** — Anything found during work that's out of scope becomes a new issue

## Review Criteria
Every PR review answers three questions:
1. **Does this work?** — Tests pass, logic is correct, edge cases handled
2. **Does this adhere to SOLID principles?** — Single responsibility, open/closed, Liskov, interface segregation, dependency inversion
3. **Would I merge this into production?** — Code quality, readability, no shortcuts that create tech debt

If any answer is "no," the PR gets feedback and goes back for fixes.

## Conventions
- Small, reviewable PRs (one logical change per PR)
- Commit messages: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`
- All PRs must pass `cargo test` before merge
- Issues are prioritized on the GitHub Project board
