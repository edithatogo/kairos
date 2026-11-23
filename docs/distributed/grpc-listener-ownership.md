# Tonic listener ownership

## Decision

`TonicTransport::from_listener` starts a server on a bound
`std::net::TcpListener`. Reserve the listeners and read their addresses
before configuring a group of peers. Hand each listener to its transport
for use through shutdown. This keeps the port allocated between address
discovery and server startup.

The address-based `TonicTransport::bind` still accepts the same address,
peer list, and config. It opens a listener and delegates to the shared
startup path. Both constructors use the same service, protocol, and
shutdown behavior. A failed startup drops the owned listener.

## Network scope

The address and exposure come from the caller's listener. The same peer
validation, message limits, and runtime configuration apply after handoff.
The Tonic transport oracles use local loopback addresses.

## Conformance

The Tonic oracles exercise a listener through handoff, shutdown and port
reuse. Pair tests reserve both listeners before starting the peers. They
continue to check optimistic ordering, migration acknowledgement,
full-width timestamps, and GVT agreement. The recovery test reserves the
peer address while withholding its RPC service, then starts the service and
checks that a heartbeat arrives.
