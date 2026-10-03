# Accepted compiler-bound local smoke

Source68b8d7a311b7acaa099a0f1c4aec4933f4b98a23, absolute Rust1.98.1 Cargo/rustc binaries and hashes, Cargo cache identity, unchanged configuration and16 source hashes. Root12 collector tests and strict collector passed; independent review reproduced inputs and verified raw log/source/tool hashes. Seed482027, one warmup and5 alternating samples per sparse/dense4/8-LP case, all sequential/conservative/optimistic parity. Evidence SHAe7d08f74b0c448cac281cd95199e24cf9d34218a303a80ad0a66f8e1323cbc8c.

Only runtime run calls are timed. Construction/scheduling, extraction, parity validation and fossil collection are excluded. This tiny lightweight-handler single-host ring shows substantial timing variation; it does not prove general speedup/scaling, simultaneous CPU execution, distributed rollback or HPC acceptance. Earlier ec9828e raw files are preserved separately and superseded for pinned compiler provenance.
