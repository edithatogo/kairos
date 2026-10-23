# syntax=docker/dockerfile:1.27

FROM rust:1.99.1-alpine@sha256:b1d5dd5a22a09404d3e798c0cf051b689077360374eb505fa5bb93c479693d87 AS builder
WORKDIR /src
RUN apk add --no-cache musl-dev
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --locked --release -p kairo-ecs-cli

FROM python:3.15.1-alpine3.24@sha256:54448f67d7c31c39e5cdd3ea2d6382395ce142806c284425f8e897d78a0465b3
RUN apk add --no-cache ca-certificates tzdata bash coreutils
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
RUN python3 -m venv .venv \
  && .venv/bin/python -c 'import sys; assert sys.version_info[:3] == (3, 15, 1); assert sys.prefix == "/work/.venv"'
ENV VIRTUAL_ENV=/work/.venv
ENV PATH="/work/.venv/bin:${PATH}"
USER kairo
ENTRYPOINT ["/usr/local/bin/kairo-ecs-entrypoint"]
CMD ["--help"]
