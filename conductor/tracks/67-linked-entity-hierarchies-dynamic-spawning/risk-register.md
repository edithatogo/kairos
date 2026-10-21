# Track 67 Risk Register

Severity scale: Low (1), Medium (2), High (3), Critical (4), Release Blocker (5).

| Risk | Likelihood | Impact | Severity | Mitigation |
|---|---|---|---|---|
| Dynamic spawning during iteration invalidates archetype memory buffers | Low (1) | Critical (4) | High (3) | Use deferred command buffer / command queue pattern for spawning during tick execution |
| Orphaned child entities leak memory when parent is despawned | Medium (2) | High (3) | High (3) | Apply explicit detach/reparent/cascade policy with conservation assertions; independently surviving children must not be destroyed automatically |
| Circular parent-child references cause infinite traversal loops | Low (1) | High (3) | Medium (2) | Reject cycles in ownership hierarchies; define traversal bounds for other typed relationship graphs |
