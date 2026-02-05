FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /workspace

FROM chef AS planner
RUN --mount=type=bind,source=.,target=/workspace \
    cargo chef prepare --recipe-path /root/recipe.json

FROM chef AS builder
ENV CARGO_TARGET_DIR=/root/
COPY --from=planner /root/recipe.json /root/recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path /root/recipe.json
RUN --mount=type=bind,source=.,target=/workspace \
    cargo build --release --target x86_64-unknown-linux-musl --bins

FROM alpine AS runtime
RUN <<-EOF
    addgroup -S hcard
    adduser -S hcard -G hcard
EOF
COPY --from=builder /root/x86_64-unknown-linux-musl/release/hcard /usr/local/bin/
USER hcard
EXPOSE 80
STOPSIGNAL SIGQUIT
ENTRYPOINT ["/usr/local/bin/hcard"]
