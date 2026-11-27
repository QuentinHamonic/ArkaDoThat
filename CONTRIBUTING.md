# Contributing to ArkaDoThat

ArkaDoThat is a local-first project maintained by one person. The repository is public so people can use and inspect it, but general external contributions are not open by default. The maintainer may invite a focused contribution when it fits the roadmap. Development and release decisions follow the [ArkaDoThat delivery workflow](docs/WORKFLOW.md).

Please do not open an unsolicited pull request. For a suspected security issue, follow [SECURITY.md](SECURITY.md). For a normal bug or request, an issue is useful, but it does not create an expectation of implementation or acceptance.

## Before opening a change

Read the README, docs/SPEC.md and docs/WORKFLOW.md. For a behavior change, write the user-visible outcome and its proof before coding. Keep the change focused on one observable slice.

## Pull requests

Create the branch from an up-to-date dev and open a pull request back to dev. The pull request must describe the behavior, tests run, documentation impact and any unverified platform check. Complete the self-review checklist in the pull request template.

GitHub does not count the author's own approval as an independent review. The solo maintainer path therefore uses a mandatory pull request, green CI and an explicit self-review checklist. No second maintainer or community approval is assumed at this stage.

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
