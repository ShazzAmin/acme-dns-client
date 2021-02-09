FROM rust:1.49.0 AS builder
WORKDIR /usr/src/acme-dns-client
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
COPY --from=builder /usr/local/cargo/bin/acme-dns-client /usr/local/bin/acme-dns-client
ENTRYPOINT [ "acme-dns-client" ]
