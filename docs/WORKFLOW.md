# ArkaDoThat delivery workflow

This document is the method for taking a change from an idea to a merged change or a released version. The repository is public, but development remains led by one maintainer for now. The workflow keeps that private working style while leaving a clear record for users and future readers.

## The flow

Every change follows these gates in order.

1. **Plan** — Describe the user-visible outcome, the entry point, the expected effect, the failure that matters and the proof that will verify it. Put the acceptance criteria in the issue, TODO.md or the relevant specification.
2. **Branch** — Start from an up-to-date origin/dev. Use feat/, fix/, docs/, test/, chore/ or release/ followed by a short kebab-case name. Keep one coherent change per branch.
3. **Build** — Implement the smallest observable slice. Keep TODO.md readable and diffable. Update the specification, decision record or user documentation when the behavior changes.
4. **Verify** — Run the checks required by the affected surface. Record anything that could not be run, especially platform-specific or native desktop checks.
5. **Review** — Open a pull request against dev. Read the complete diff from the base branch, inspect the tests and documentation, and complete the self-review checklist. The author’s own GitHub approval is not treated as an independent review.
6. **Independent review** — Launch the read-only independent review session described below. Give it only the work scope, the files to inspect and the verification evidence. Resolve material findings and rerun the affected checks.
7. **Merge** — Merge only after the required checks are green, the checklist is complete, the independent review has been considered and no unresolved review conversation remains. Use squash merging for feature, fix, documentation and test branches.
8. **Release** — When a version plan is ready, create a release/vX.Y.Z branch from dev, freeze the scope, run the release checklist, establish or update the stable main branch and merge the release into it. Tag the exact commit and publish the release notes.
9. **Maintain** — Create hotfix branches from the stable tag or main, merge the fix into the stable branch, then bring the same fix back into dev.

## Branch rules

dev contains the next integrated version and is the default branch until the first packaged release. main is reserved for stable releases and may remain minimal before that point. Direct pushes to dev and main are avoided. Pull requests are the record of the change, its checks and its review decision.

The local branch must be synchronized before branching.

~~~powershell
git fetch --prune origin
git switch dev
git pull --ff-only origin dev
git switch -c feat/short-name
~~~

Do not use old local backup branches as a base. Keep them only until the corresponding work is confirmed in the public history.

## GitHub merge gate

The current protection on dev requires a pull request, the six CI jobs below, resolved conversations, linear history, no force-push and no branch deletion.

- Frontend (type check + build)
- UI journey (Playwright)
- Rust (ubuntu-latest)
- Rust (windows-latest)
- Secret scan (full history) — GitLeaks
- Dependency audit

The required approval count is zero while the project has one maintainer. This is intentional because GitHub refuses to count the author's own approval as an independent review. The pull request template and the solo review checklist are the review gate in this phase. No community review or external contribution is assumed. When an independent maintainer is explicitly added, change the required approval count to one and keep stale-review dismissal enabled.

## Commit rules

Use a short imperative subject with one of the project types feat, fix, docs, test, refactor, chore or ci.

~~~text
feat(proposals): show the before and after titles
~~~

A commit should explain one coherent change. Generated binaries, local configuration, secrets and personal TODO.md files do not belong in the repository.

## Verification gates

Run the smallest relevant checks while working, then run the complete set before opening the pull request.

~~~powershell
npm run check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
node scripts/mcp-smoke.mjs target/debug/taskhud-mcp.exe
node scripts/functions-smoke.mjs target/debug/taskhud-functions.exe
git diff --check
~~~

Run python qa/test_todo_ui.py when Python Playwright and Chromium are installed. A check that was not run stays marked as unverified in the pull request.

## Solo maintainer review

The solo path is deliberately explicit because GitHub refuses to count the author's own approval as an independent review.

- Open the pull request before merging.
- Wait for every applicable CI job.
- Read the diff from dev, including deleted and untracked files that will be committed.
- Check the user-visible path, the failure path, tests, documentation, license and generated-file rules.
- Resolve every conversation and record any unverified platform check.
- Merge only after the checklist is complete.

When a second maintainer is available, enable one required approval, dismiss stale approvals after new pushes and require code-owner review for security-sensitive files.

Before the first packaged release, advance main to the release candidate commit if it is still the initial placeholder branch, set main as the default branch in GitHub and apply the same protection rules. Keep dev as the integration branch.

The one-time bootstrap is explicit. From a clean, up-to-date dev branch, create main from the same commit if the remote branch does not exist. If the placeholder main already exists, advance it with a fast-forward or a pull request because its initial commit is an ancestor of dev.

~~~powershell
git fetch --prune origin
git switch dev
git pull --ff-only origin dev
git push origin origin/dev:main
gh repo edit QuentinHamonic/ArkaDoThat --default-branch main
~~~

Apply the main protection before opening the first release pull request. Verify the remote state with:

~~~powershell
gh api repos/QuentinHamonic/ArkaDoThat/branches/main/protection
~~~

## Independent review at the end of a slice

The independent review is a second model session created with no conversation history. It uses the same model and reasoning effort as the working session, receives the smallest useful brief and reads the current workspace and diff itself. It is read-only and must not edit, commit, push or merge.

The brief identifies the outcome, the changed files and the checks already run. The reviewer reports each finding with a severity, a concrete file or behavior, the evidence and a correction proposal. It also reports what it found sound. The working session decides whether each finding is valid, fixes material findings and reruns the affected checks before presenting the work as complete.

This review is an engineering critique, not a GitHub approval and not a replacement for an independent human maintainer. A missing review session or an unresolved material finding keeps the slice incomplete.

## Change categories

An issue or task is ready to implement when its expected behavior and proof are written down. It is ready to merge when the implementation, tests, documentation and review checklist are complete. It is ready to release only when the release checklist in docs/RELEASES.md is complete.
