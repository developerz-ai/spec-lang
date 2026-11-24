# 007 Packages over git

**Status:** accepted, 2026-11-24. Nothing is built.

**Decision.** A package is a git repository and a tag. The lock file pins a commit and a content hash. No central registry, no install scripts, no build hooks. An index may list public tags; it never serves code.

**Rejected.** *npm*: search and names for free, but install scripts, a second identity system and a host that decides what a name means. *An own registry*: a service to run, secure and moderate before there is a single user. *Vendoring only*: no path for updates.

**Cost accepted.** Discovery, name disputes and abandoned packages are left to people for now. Distributing the engine itself is separate: npm and native binaries ([20](../idea/20-roadmap.md)).

**Revisit when** finding packages is a real problem, or a host failure breaks builds despite mirrors.
