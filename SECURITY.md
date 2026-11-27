# Security policy

ArkaDoThat is local-first software. It does not require an account, a cloud service or telemetry for its documented workflow.

## Automated secret scan

Every pull request and push to the integration or stable branches runs GitLeaks against the complete Git history. The repository keeps the CI scan configuration in `.gitleaks.toml`; GitLeaks is not a local development prerequisite.

## Supported code

The latest commit on dev is the actively maintained development line. Older commits and unreleased local builds may not receive fixes. A stable support policy will be published with the first packaged release.

## Reporting a vulnerability

Please do not open a public issue for a vulnerability. Use a [private GitHub security advisory](https://github.com/QuentinHamonic/ArkaDoThat/security/advisories/new) when available. If that form is unavailable, contact the maintainer through the [GitHub profile](https://github.com/QuentinHamonic) and state that the report contains security-sensitive details. Include the affected commit or version, reproduction steps, impact and a suggested mitigation. Do not include real credentials or personal project data.

The maintainer will acknowledge a report when it has been reviewed and will coordinate disclosure after a fix or mitigation is available.
