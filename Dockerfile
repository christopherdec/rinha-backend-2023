FROM rust:1.83.0-slim-bookworm AS build

ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN cargo new --bin app
WORKDIR /app

COPY Cargo.toml /app/
COPY Cargo.lock /app/
RUN cargo build --release

COPY src /app/src
RUN touch src/main.rs
RUN cargo build --release

FROM debian:bookworm-slim

COPY --from=build /app/target/release/rinha-backend-2023 /app/rinha

CMD "/app/rinha"
