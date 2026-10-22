# Security policy

agentstat handles account tokens and usage data. Credential disclosure, requests
sent to an unintended origin, scope confusion, and unsafe local process behavior
are security concerns.

## Reporting a vulnerability

Do not post exploit details, tokens, cookies, credential files, or private account
payloads in public issues or pull requests.

If **Security → Report a vulnerability** is available on this repository, use
that private reporting form. Otherwise, use a contact method the repository
owner provides on their [GitHub profile](https://github.com/dhanushk-offl).
If no private channel is available, open an issue titled **Private security
contact requested** with no vulnerability details. A maintainer can then arrange
a private reporting channel.

Once a private channel is available, include:

- The affected crate version or commit and enabled features.
- The provider, platform, and credential type involved.
- Reproduction steps using synthetic credentials where possible.
- The expected behavior, observed behavior, and potential impact.
- A proposed fix, if you have one.

If real credentials have been exposed, revoke or rotate them through the provider
and remove them from any shared material. Maintainers will assess reports, work
on a fix where appropriate, and coordinate disclosure. There is no guaranteed
response time or bounty program.

## Supported versions

The project is in its initial `0.1` development stage. Security fixes target the
current main branch and latest release when available. Older revisions do not
have a separate maintenance or backport commitment.

Ordinary provider authentication failures and API schema changes can be reported
as bugs with sanitized reproduction details; use the private process when a
security impact is involved.
