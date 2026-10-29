# Local output directory guard

This narrow native boundary checks an already-open directory without changing
its permissions. The Arrow crate remains unsafe-free. The native call borrows
the OS handle and does not transfer ownership.

The current OS identity and administrators are trusted. The contract excludes
arbitrary writers sharing that identity, changes by trusted users after a check,
and hostile filesystem servers. It does not authenticate a pathname's historical
identity before acquisition or keep its ancestors immutable after return.

Accepted profiles:

- Linux: current-UID ownership, no group/other write permission, and a local
  ext-family, XFS, btrfs, tmpfs or overlay filesystem. POSIX ACL named-user/group
  permissions are constrained by the group mask represented in the mode bits.
  A private run additionally requires no group/other permissions.
- macOS: current-UID ownership, equivalent mode checks, local APFS, and no
  extended ACL on the descriptor. ACL presence is queried with `fstatx_np` and
  `filesec_query_property`, not inferred from `acl_get_fd_np`'s errno. Present
  ACLs, including harmless or empty extended ACL records, are conservatively
  rejected rather than interpreted as permission to publish.
- Windows: current-token ownership, local NTFS/ReFS with persistent ACLs, a
  valid non-null DACL, and no effective or inherited grant of mutation rights
  to identities other than the current identity, SYSTEM and Administrators.
  Private runs additionally reject read/traverse grants to other identities.
  Unsupported ACE forms are rejected. File-only inherit-only grants are checked
  because they can become effective on staging files.

Other filesystem models and unavailable native permission evidence return an
explicit error. These checks do not make FNV content hashes cryptographic or
establish crash durability; the publisher has separate file/directory sync rules.
Platform source compilation and platform runtime qualification are distinct.

