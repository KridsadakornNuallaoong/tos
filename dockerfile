FROM rust:1.90-alpine AS builder

RUN apk add musl-dev

WORKDIR /usr/src/app

COPY Cargo.toml ./

COPY ./src ./src

RUN cargo fetch

RUN cargo build --release

FROM alpine:latest

COPY --from=builder /usr/src/app/target/release/tos /usr/local/bin

ENTRYPOINT [ "tos" ]