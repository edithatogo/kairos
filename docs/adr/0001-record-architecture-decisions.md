# ADR 0001: Record Architecture Decisions

## Status

Accepted

## Context

KairoECS spans multiple languages and compatibility surfaces. Design decisions need to be reviewable and durable.

## Decision

Use ADRs for changes to scheduler semantics, ECS model, FFI ABI, Arrow schemas, public APIs, release policy, package naming, and compatibility promises.

## Consequences

Subagents can work in parallel, but contract-changing PRs require explicit review and documentation.
