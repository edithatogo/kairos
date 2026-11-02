# CAP-48.1 alternate-host route

Maturity: qualified host/transport prerequisite; distributed rollback acceptance remains open.

The NAS was intended as an independent Linux worker alongside the Mac, for real
cross-host rollback/anti-message/GVT, injected delay/duplication/failure, restart
and deterministic serial parity. Only read-only SSH discovery ran there; it
never returned remote host/toolchain information. No NAS worker was activated.

Google Colab CLI0.7.5 successfully allocated an isolated CPU runtime and refreshed
its existing client-owned OAuth credentials. No new login/consent, GPU, purchase,
Drive mount or shared-session takeover was needed. Colab offers a suitable
alternate Linux test host; NAS connectivity is no longer the sole host route.

The runtime read back Linux6.6.159+, x86_64, two CPUs, OpenMPI4.1.6 and curl.
Its managed control kernel is Python3.13.18. A private uv0.13.6 copy was required
because image uv0.13.5 lacked Python3.15.2 metadata; the repository's actual
`/content/kairos-cap48/kairos/.venv/bin/python` then verified3.15.2 and the exact
repo-local prefix. The kernel was preserved. Rustup1.29.2 was installed from its
official versioned archive after SHA256 verification, and actual Rust1.99.1 was
read back. Mac and Linux compiler LLVM builds differ and are recorded separately.
The external CLI's retained tool-owned Python3.14.8 is distinct from repo Python.

An authenticated Colab exec supplied the VM's public SSH host key. The Mac
pinned that key in a lane-owned known-hosts file and used StrictHostKeyChecking;
no global SSH configuration or private key was copied. SSH reverse forwarding
carried a Linux curl HTTP/2 gRPC Heartbeat into the Mac's exact-source Tonic
endpoint. HTTP200, grpc-status0 and an accepted LP99 acknowledgement were
verified from the framed protobuf bytes. This proves two-host protocol
connectivity only, not admission/retirement, rollback or safe distributed GVT.

The first local control socket path exceeded macOS's Unix socket limit; no RPC
ran in that attempt. The corrected private short socket path succeeded. Native
session/credential metadata stay ignored and private under the lane's artifact
directory; only sanitized facts and hashes may enter source receipts.

The frozen0718b9c affected-three-crate locked all-feature suite passed on Colab,
followed by actual native MPI smoke with two and four ranks. Missing libclang
initially blocked bindgen; installing libclang-dev on the owned VM resolved that
prerequisite. A separately hashed final store packet passed182 all-feature PDES
tests on each of Mac and Colab, including real process-death recovery. See
[store evidence](../../conductor/tracks/48-time-warp-optimistic-rollback-runtime/evidence/store-20261103/receipt.json).

Full native logs were retrieved before teardown. CLI stop succeeded for only
`kairos-cap48-20261103`; fresh server readback reported no active sessions. The
Mac probe and forwarding were already stopped/reaped. The VM's overlay filesystem
qualifies the recorded process-kill scenarios, not physical power-loss durability.

The fixed participant transaction-store candidate is qualified privately; the
next implementation is the complete portable engine image,
fixed verifier/driver, admission/retirement and authenticated cut/restart join.
Process-local native authority witnesses must not be sent over this route as
remote proof. Colab's ephemeral VM is not a durable production deployment or a
power-loss qualification target. Cross-host native MPI additionally needs a
reviewed compatible MPI installation; Mac OpenMPI5 and Colab OpenMPI4 are not
claimed wire-compatible.
