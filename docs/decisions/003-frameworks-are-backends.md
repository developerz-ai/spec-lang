# 003 Frameworks are backends

**Status:** accepted, 2026-11-24.

**Decision.** Lower to the highest-level target that preserves the declared guarantees. Ultimate first; anygame when its schema, ECS and headless runner exist. A backend that cannot keep a guarantee refuses.

**Rejected.** *Raw languages first*: every framework's hard-won behaviour would be rebuilt before the first useful day. *Plugins that run inside a bundler*: invisible to the app's own checks and to agents reading it.
