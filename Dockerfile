FROM rust:1.80 AS  builder

COPY Cargo.lock Cargo.toml /workdir/
COPY src /workdir/src

WORKDIR /workdir

RUN apt-get update && \
    apt-get install -y libssl-dev
RUN cargo build --release

FROM ubuntu:latest

RUN apt-get update && \
    apt-get install -y curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /workdir/target/release/dynuupdater /usr/bin/dynuupdater

USER nobody

ENTRYPOINT ["/usr/bin/dynuupdater"]
