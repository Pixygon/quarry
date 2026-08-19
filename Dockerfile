# Build the Quarry, then run it on a slim base. The vendored crates make the
# build self-contained (Pixygon/Infinite is private).
FROM rust:1-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY vendor ./vendor
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/quarry /usr/local/bin/quarry
ENV PORT=3000 QUARRY_DATA=/data
VOLUME ["/data"]
EXPOSE 3000
CMD ["quarry"]
