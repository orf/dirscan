FROM rust:1.42 as build
WORKDIR /build
COPY Cargo.lock Cargo.toml ./
COPY src/ ./src/
RUN cargo build --release

FROM debian:stretch-slim
COPY --from=build /build/target/release/dirscan .
CMD ["dirscan"]

