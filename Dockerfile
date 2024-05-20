FROM rust:1.78-alpine as builder

RUN apk add libc-dev

WORKDIR /usr/src/app
COPY . .
RUN cargo install --path .

FROM alpine:3.19
COPY --from=builder /usr/local/cargo/bin/rust-unix-relay-server /usr/local/bin/rust-unix-relay-server
CMD ["rust-unix-relay-server","/tmp/sockets/relay"]