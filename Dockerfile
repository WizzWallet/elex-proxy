FROM rustlang/rust:nightly as builder

WORKDIR /app

COPY . .

RUN cargo install --path .

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/local/cargo/bin/elex-proxy /usr/local/bin/elex-proxy

CMD ["elex-proxy"]
