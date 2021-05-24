# Dockerfile for OriginTrail Parachain
# Note: This is currently designed to simplify development

FROM debian:stable

ARG PROFILE=release

# show backtraces
ENV RUST_BACKTRACE 1

# Updates core parts
RUN apt-get update -y && \
	apt-get install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev curl

# Download Starfleet parachain repo
RUN git clone --branch develop https://github.com/OriginTrail/starfleet-parachain
WORKDIR /starfleet-parachain

# Install rust and build node
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y && \
	export PATH="$PATH:$HOME/.cargo/bin" && \
	rustup toolchain install nightly && \
	rustup target add wasm32-unknown-unknown --toolchain nightly && \
	rustup default stable && \
	cargo build "--$PROFILE"

# 30333 for p2p traffic
# 9933 for RPC call
# 9944 for Websocket
# 9615 for Prometheus (metrics)
EXPOSE 30333 9933 9944 9615

ENV PROFILE ${PROFILE}

# Start compiled Starfleet bootnode
CMD ./target/release/origintrail-parachain --base-path /tmp/parachain \
    ${ADDITIONAL_PARAMS}

