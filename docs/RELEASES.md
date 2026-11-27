# TaskHUD release planning

TaskHUD uses Semantic Versioning. Until the first stable release, the 0.x series may change behavior between minor versions. A release is a named, reviewable plan rather than a push made when the tree happens to look ready.

## Release states

Unreleased is work merged into dev after the last tag. A release plan is a file under docs/releases/. A release candidate is a frozen release/vX.Y.Z branch. A published release is an immutable vX.Y.Z tag with GitHub release notes and attached artifacts when packaging is supported.

## Release plan template

Create docs/releases/vX.Y.Z.md from this checklist.

### Scope

- User-visible changes included
- Known limitations carried forward
- Explicitly deferred work

### Before the release branch

- [ ] The scope is agreed and linked to issues or specification sections.
- [ ] The working tree is clean and dev is up to date.
- [ ] The version is chosen according to Semantic Versioning.
- [ ] The changelog and README status are ready to update.

### On the release branch

- [ ] package.json and every Cargo manifest carry the same version.
- [ ] src-tauri/tauri.conf.json carries the same version.
- [ ] npm run check passes.
- [ ] cargo fmt --all -- --check passes.
- [ ] cargo clippy --workspace --all-targets -- -D warnings passes.
- [ ] cargo test --workspace passes.
- [ ] Connector smoke tests pass on the supported platforms.
- [ ] The browser UI journey passes with the required Playwright environment.
- [ ] The native desktop journey is observed on each platform claimed by the release.
- [ ] The secret scan and dependency review pass.
- [ ] The license, README, documentation and known limitations are accurate.

### Publication

- [ ] Open a pull request for the release branch.
- [ ] Complete the solo review checklist or obtain the required independent approval.
- [ ] If main does not exist, create it once from the current dev commit. If it is still the initial placeholder branch, advance it with a fast-forward or pull request.
- [ ] Protect main with the same required checks and set it as the default branch.
- [ ] Merge the release branch into main.
- [ ] Create the annotated tag vX.Y.Z on the exact release commit.
- [ ] Create the GitHub release with the scope, verification evidence, limitations and artifacts.
- [ ] Merge or cherry-pick any release-only fixes back into dev.

## Current plan

The first packaged release is not published yet. The working plan is tracked in docs/releases/v0.1.0.md. It must not be described as released until the packaging and native journey checks are complete.
