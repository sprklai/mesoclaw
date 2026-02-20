# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.x.x (current) | ✅ Active |
| < 0.1.0 | ❌ End of life |

## Reporting a Vulnerability

**Please do NOT open a public GitHub issue for security vulnerabilities.**

Use [GitHub Security Advisories](../../security/advisories/new) to report vulnerabilities privately. This ensures the issue can be assessed and a fix prepared before public disclosure.

### What to Include

- **Description:** Clear explanation of the vulnerability
- **Impact:** What an attacker can achieve
- **Reproduction:** Step-by-step instructions
- **Version:** MesoClaw version affected
- **Environment:** OS, configuration details
- **Suggested fix:** (optional) Your proposed remediation

### Proof of Concept

Including a minimal proof-of-concept helps us understand and reproduce the issue faster. We ask that you:
- Limit testing to your own systems
- Not exploit the vulnerability beyond what is needed to demonstrate it
- Not access or modify data that does not belong to you

## Response SLAs

| Milestone | Timeline |
|-----------|----------|
| Acknowledgment | 48 hours |
| Initial assessment | 1 week |
| Fix — Critical | 2 weeks |
| Fix — High | 30 days |
| Fix — Medium | 90 days |
| Fix — Low | Next release cycle |

## Disclosure Policy

We follow **coordinated disclosure**:

1. Reporter submits via GitHub Security Advisories
2. We acknowledge within 48 hours
3. We investigate and develop a fix
4. We coordinate a release date with the reporter
5. We publish a security advisory with the fix
6. We credit the reporter in the advisory (unless they prefer anonymity)

We ask reporters to keep the vulnerability confidential until a fix is released.

## Security Scope

### In Scope

- Authentication and authorization bypasses
- Data exposure (API keys, credentials, memory content)
- Remote code execution via agent tool execution
- Path traversal in file system operations
- Injection attacks in shell command execution
- Security policy bypass in the agent loop
- Privilege escalation

### Out of Scope

- Vulnerabilities in third-party AI providers (report to them directly)
- Issues requiring physical access to the device
- Social engineering attacks
- Denial-of-service via rate limiting (expected behavior)
- Issues in dependencies (report to the dependency maintainer)

## Security Contacts

Security advisories: Use [GitHub Security Advisories](../../security/advisories/new)

For questions about the security policy itself, open a regular GitHub issue with the `question` label.

## Hall of Fame

We maintain a list of security researchers who have responsibly disclosed vulnerabilities. Reporters will be credited in security advisories unless they request anonymity.
