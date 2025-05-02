FROM rust:1.86 as builder

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock* ./

RUN mkdir src && \
	echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
	cargo build --release && \
	rm -rf src

COPY src ./src
COPY .env ./

RUN touch src/main.rs && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl-dev && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/backend /app/
COPY --from=builder /usr/src/app/.env /app/

EXPOSE 8080

CMD ["./backend"]
