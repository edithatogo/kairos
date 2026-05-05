# C# Style Guide

- Target `net10.0` and `net11.0` lanes.
- Treat .NET 10 as stable and .NET 11 as preview/GA-gated until final release.
- Use `SafeHandle` and explicit `Dispose` for native resources.
- Enable nullable reference types and warnings-as-errors for library projects.
- Test with xUnit or NUnit, coverage via coverlet, and benchmarks via BenchmarkDotNet.
- Package native assets under NuGet `runtimes/<rid>/native/`.
