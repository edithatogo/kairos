# syntax=docker/dockerfile:1.7

FROM rust:1.76-alpine AS builder
WORKDIR /src
RUN apk add --no-cache musl-dev
COPY Cargo.toml ./
COPY crates ./crates
RUN cargo build --release -p kairo-ecs-cli

FROM alpine:3.20
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
