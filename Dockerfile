# syntax=docker/dockerfile:1.4

FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static upx
# Install sentry-cli
RUN wget -qO /usr/local/bin/sentry-cli "https://sentry.io/get-cli/" && \
    chmod +x /usr/local/bin/sentry-cli
WORKDIR /app

FROM chef AS planner
COPY  . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner  /app/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json

COPY  . .

RUN cargo build --release --target x86_64-unknown-linux-musl

# Upload debug symbols to Sentry before stripping
ARG SENTRY_AUTH_TOKEN
ARG SENTRY_ORG
ARG SENTRY_PROJECT
ARG VERSION

RUN if [ -n "$SENTRY_AUTH_TOKEN" ]; then \
        sentry-cli debug-files upload \
            --auth-token ${SENTRY_AUTH_TOKEN} \
            --org ${SENTRY_ORG} \
            --project ${SENTRY_PROJECT} \
            --include-sources \
            /app/target/x86_64-unknown-linux-musl/release/cloudflare-access-webhook-redirect; \
    fi

# Strip and compress after uploading symbols
RUN strip --strip-all /app/target/x86_64-unknown-linux-musl/release/cloudflare-access-webhook-redirect && \
    upx --best --lzma /app/target/x86_64-unknown-linux-musl/release/cloudflare-access-webhook-redirect

FROM alpine:3.20 AS env

# mailcap is used for content type (MIME type) detection
# tzdata is used for timezones info
RUN apk update && \
    apk upgrade --no-cache && \
    apk add --no-cache ca-certificates mailcap tzdata

RUN update-ca-certificates

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "10001" \
    "appuser"

FROM scratch AS runtime

ARG version=unknown
ARG release=unreleased
ARG vendor=unknown

LABEL org.opencontainers.image.version="${version}" \
      org.opencontainers.image.revision="${release}" \
      org.opencontainers.image.vendor="${vendor}" \
      org.opencontainers.image.title="cloudflare-access-webhook-redirect"

COPY --from=env  /etc/passwd /etc/passwd
COPY --from=env  /etc/group /etc/group
COPY --from=env  --chmod=444 /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=env  --chmod=444 /usr/share/zoneinfo /usr/share/zoneinfo

WORKDIR /app
COPY --from=builder --chmod=555 /app/target/x86_64-unknown-linux-musl/release/cloudflare-access-webhook-redirect ./app

USER 10001:10001

ENTRYPOINT ["./app"]
