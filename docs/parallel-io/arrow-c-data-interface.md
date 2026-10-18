# Arrow C Data Interface & Zero-Copy FFI Spine in KairoECS

This document details the zero-copy C Data Interface integration ( and ) and Arrow/Parquet checkpoint serialization in KairoECS.

## Zero-Copy Ownership and 64-Byte Alignment
- All memory buffers exported via the C Data Interface adhere to 64-byte hardware alignment requirements for SIMD / AVX-512 vectorization and GPU direct memory access.
- Ownership rules follow the official Apache Arrow C Data Interface specification: memory is managed by Rust (or the exporting engine) until explicitly released via the callback function pointer ().

## Release Callbacks & Idempotence
- Exported structs (, ) provide idempotent  callbacks (, ). Calling release multiple times is safe and avoids double-free bugs.

## PyCapsule Interoperability
- Exported pointers map directly to Python PyCapsules ( and ), enabling zero-copy ingestion and egress across Polars, DuckDB, and PyArrow without data serialization overhead.
