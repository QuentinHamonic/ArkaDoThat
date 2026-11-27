# Contributing to TaskHUD

TaskHUD is a local-first, maintainer-led project. The repository is public so people can use and inspect it. Development and release decisions follow the TaskHUD delivery workflow in docs/WORKFLOW.md.

## Before opening a change

Read the README, docs/SPEC.md and docs/WORKFLOW.md. For a behavior change, write the user-visible outcome and its proof before coding. Keep the change focused on one observable slice.

## Pull requests

Create the branch from an up-to-date dev and open a pull request back to dev. The pull request must describe the behavior, tests run, documentation impact and any unverified platform check. Complete the self-review checklist in the pull request template.

GitHub does not count the author's own approval as an independent review. The solo maintainer path therefore uses a mandatory pull request, green CI and an explicit self-review checklist. When an independent maintainer is available, the repository can require an approving review.

Before merge, run the independent end-of-slice review described in docs/WORKFLOW.md. It is a read-only critique with minimal context. Consider its findings and rerun the affected checks.

## Local checks

Run the checks that cover the change, then the complete set before merge.

~~~powershell
npm run check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
git diff --check
~~~

The connector smoke tests and the Playwright journey are required when the changed surface affects them. See docs/WORKFLOW.md for the commands and evidence rules.

## Commits

Use a short imperative subject with a project type such as feat, fix, docs, test, refactor, chore or ci. Do not commit secrets, generated binaries, local configuration or personal project files.

## License

Contributions are released under the GNU General Public License v3 or later, as described in LICENSE.
