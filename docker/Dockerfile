# syntax=docker/dockerfile:1.7

FROM rust:1.76-alpine@sha256:e594a9705c4514c0e0b5ed2409f7ec34f20af09a33d242524281247b74196c43 AS builder
WORKDIR /src
RUN apk add --no-cache musl-dev
COPY Cargo.toml ./
COPY crates ./crates
RUN cargo build --release -p kairo-ecs-cli

FROM alpine:3.20@sha256:d9e853e87e55526f6b2917df91a2115c36dd7c696a35be12163d44e6e2a4b6bc
RUN apk add --no-cache ca-certificates tzdata bash coreutils python3
COPY --from=builder /src/target/release/kairo-ecs-cli /usr/local/bin/kairo-ecs-cli
COPY docker/entrypoint.sh /usr/local/bin/kairo-ecs-entrypoint
COPY docker/telemetry-plugin/cloud-output.py /usr/local/bin/kairo-ecs-cloud-output
RUN addgroup -S kairo \
  && adduser -S -G kairo kairo \
  && mkdir -p /var/lib/kairo/output /work \
  && chown -R kairo:kairo /var/lib/kairo /work \
  && chmod +x /usr/local/bin/kairo-ecs-entrypoint /usr/local/bin/kairo-ecs-cloud-output
ENV KAIRO_OUTPUT_DIR=/var/lib/kairo/output
WORKDIR /work
USER kairo
ENTRYPOINT ["/usr/local/bin/kairo-ecs-entrypoint"]
CMD ["--help"]
