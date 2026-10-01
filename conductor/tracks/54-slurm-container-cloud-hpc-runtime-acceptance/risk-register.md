# Track 54 Risk Register

Severity scale: Low (1-2), Medium (3-4), High (5-6), Critical (7-10).

| Risk | Impact | Mitigation |
|---|---|---|
| Offline validation is mistaken for runtime proof | False cloud/HPC claim | Require live job IDs and output artifacts |
| Provider quota blocks canaries | Track cannot close | Record explicit quota blocker and keep status non-Done |
| Container runs wrong code revision | Invalid evidence | Require commit SHA and image digest |
| Slurm job omits MPI/GPU path | Incomplete acceptance | Separate CPU, MPI, and GPU job templates |
| Cloud credentials leak | Security incident | Use protected environments and sanitized evidence |
