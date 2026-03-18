FROM rust:1.85-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    cmake pkg-config libssl-dev git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs && cargo build --release && rm -rf src

COPY src/ src/
RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 git && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/gitnote-api /usr/local/bin/gitnote-api

RUN mkdir -p /data
ENV DATABASE_URL=/data/gitnote.db
ENV GIT_REPOS_PATH=/data/repo.git
ENV PORT=8080

EXPOSE 8080
CMD ["gitnote-api"]
