# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in Sans, please report it responsibly.

**Email:** security@sans.dev

**What to include:**
- Description of the vulnerability
- Steps to reproduce
- Impact assessment
- Suggested fix (if any)

**Response timeline:**
- Acknowledgment within 48 hours
- Initial assessment within 7 days
- Fix timeline communicated within 14 days

**What we consider security issues:**
- Compiler crashes or segfaults on any input (denial of service)
- Memory safety violations (use-after-free, buffer overflow) in compiled programs
- Sandbox escapes in the web playground
- Hash collision attacks against Map/JSON internals
- Server vulnerabilities (request smuggling, header injection, etc.)

**What is NOT a security issue:**
- Bugs that require the attacker to have local code execution (Sans compiles to native code — if you can run Sans code, you already have code execution)
- Feature requests or general bugs (use GitHub Issues)
- Vulnerabilities in dependencies that don't affect Sans

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.8.x   | Yes       |
| < 0.8   | No        |

## Security Features

Sans includes several security features for production use:

- **Bounds checking** on array and string access (v0.8.0)
- **SIGPIPE handling** in HTTP servers (v0.8.0)
- **Panic recovery** per request via setjmp/longjmp (v0.8.0)
- **Scope-based memory management** with full reference walking (v0.8.1)
- **JSON depth limits** (512) to prevent stack overflow (v0.8.1)
- **Request size limits** and input validation on HTTP servers (v0.8.2)
- **Bounded thread pool** with connection limits (v0.8.2)
- **Graceful shutdown** on SIGTERM/SIGINT (v0.8.2)
- **SipHash** for Map/JSON to prevent hash collision DoS (v0.8.3)
- **Sandboxed playground** with Docker isolation (v0.8.4)
