FROM rust:1 AS chef
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .

# Install `dx`
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli --root /.cargo -y --force
RUN cargo binstall wasm-bindgen-cli --root /.cargo -y --force
RUN rustup target add wasm32-unknown-unknown
ENV PATH="/.cargo/bin:$PATH"

ARG SERVER_HOSTNAME
ENV SERVER_URL=https://${SERVER_HOSTNAME}
ENV WEBSOCKET_URL=wss://${SERVER_HOSTNAME}/ws

RUN cargo build -p takumi-worker --target wasm32-unknown-unknown --release
RUN wasm-bindgen target/wasm32-unknown-unknown/release/takumi_worker.wasm --out-dir /app/workers/takumi_worker --target no-modules

# Create the final bundle folder. Bundle always executes in release mode with optimizations enabled
RUN dx bundle --platform web

FROM debian:bookworm-slim AS runtime
COPY --from=builder /app/target/dx/tak/release/web/ /usr/local/app
COPY --from=builder /app/workers/ /usr/local/app/workers

# set our port and make sure to listen for all connections
ENV PORT=8080
ENV IP=0.0.0.0
ENV DB_URL=surrealdb:8000

# expose the port 8080
EXPOSE 8080

WORKDIR /usr/local/app
ENTRYPOINT [ "/usr/local/app/server" ]