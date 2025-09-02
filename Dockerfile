# syntax=docker/dockerfile:1.7

FROM rust:1.80-bookworm AS builder
WORKDIR /app

# Кэшируем зависимости
COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    bash -lc 'mkdir -p src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src'

# Сборка приложения
COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release


FROM debian:bookworm-slim AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

# Непривилегированный пользователь
RUN useradd -r -u 10001 -g root app

# Бинарник
COPY --from=builder /app/target/release/rs-port-forward /usr/local/bin/rs-port-forward

ENV RUST_LOG=info

# По умолчанию бинарь сам ищет конфиг в /etc/rs-port-forward.config.json (Linux),
# но зададим явный CMD для наглядности.
ENTRYPOINT ["/usr/local/bin/rs-port-forward"]
CMD ["--config", "/etc/rs-port-forward.config.json"]

