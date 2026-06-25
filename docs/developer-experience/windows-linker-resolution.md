# Windows Linker Resolution

Some local Windows shells place Git's Unix compatibility tools before Visual
Studio Build Tools on `PATH`. In that state, Rust's MSVC target can compile
crates but fail during test or benchmark linking because `link.exe` resolves to
Git's `usr/bin/link.exe` instead of the Microsoft linker.

Run this diagnostic before treating an MSVC `cargo test`, `cargo bench`, or
example execution failure as a code defect:

```powershell
node scripts/validation/validate-windows-linker-resolution.mjs
```

For strict gates that require the MSVC linker, run:

```powershell
node scripts/validation/validate-windows-linker-resolution.mjs --require-msvc-linker
```

A blocked result means the first `link.exe` on `PATH` is Git's shim. Open a
Visual Studio Developer PowerShell or move the Visual Studio
`VC/Tools/MSVC/.../bin/Hostx64/x64` directory before Git's `usr/bin` directory
for the shell running Cargo. Do not rewrite repository configuration to
compensate for a user-specific `PATH`; the intended repository-side control is
to detect and record the environment blocker.

If the GNU Rust toolchain passes the same tests, record that as implementation
evidence and keep the MSVC result classified as an environment blocker until
this diagnostic reports the Visual Studio linker first.
