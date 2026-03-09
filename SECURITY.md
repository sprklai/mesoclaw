# Security Policy

## Supported Versions

| Version | Supported          |
|---------|--------------------|
| 0.1.x   | Yes                |

## Reporting a Vulnerability

**Do not open a public issue for security vulnerabilities.**

Instead, please email **security@nsrtech.dev** with:

1. A description of the vulnerability
2. Steps to reproduce
3. Potential impact assessment
4. Any suggested fixes (optional)

## Response Timeline

- **Acknowledgment**: Within 48 hours
- **Initial assessment**: Within 7 days
- **Fix timeline**: Within 90 days of confirmed report
- **Public disclosure**: After fix is released, coordinated with reporter

## What Qualifies as a Security Issue

- Authentication or authorization bypass
- Remote code execution
- SQL injection or other injection attacks
- Credential exposure or leakage
- Cross-site scripting (XSS) in the web frontend
- Privilege escalation
- Denial of service via crafted input

## What Does NOT Qualify

- Bugs that do not have a security impact
- Feature requests
- Issues requiring physical access to the machine
- Social engineering attacks
- Vulnerabilities in dependencies that are already patched upstream

## Security Best Practices

- Always set `gateway_auth_token` in production deployments
- Use environment variables (`MESOCLAW_TOKEN`) instead of command-line arguments for secrets
- Run the daemon as a non-root user
- Use a reverse proxy with TLS for remote access
- Keep dependencies updated (`cargo audit`, Dependabot)
