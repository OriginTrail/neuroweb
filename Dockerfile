# This file is sourced from https://github.com/paritytech/polkadot/blob/master/scripts/ci/dockerfiles/polkadot/polkadot_builder.Dockerfile
# This is the build stage for polkadot-parachain. Here we create the binary in a temporary image.
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /origintrail-parachain
COPY . /origintrail-parachain

RUN rustup default nightly && cargo build --release

# This is the 2nd stage: a very small image where we copy the Polkadot binary."
FROM docker.io/library/ubuntu:20.04

COPY --from=builder /origintrail-parachain/target/release/origintrail-parachain /usr/local/bin
COPY --from=builder /origintrail-parachain/res/origintrail-parachain-2043-raw.json /config/origintrail-parachain-2043-raw.json

RUN useradd -m -u 1000 -U -s /bin/sh -d /origintrail-parachain origintrail-parachain && \
    mkdir -p /data /origintrail-parachain/.local/share && \
    chown -R origintrail-parachain:origintrail-parachain /data && \
    ln -s /data /origintrail-parachain/.local/share/origintrail-parachain && \
# unclutter and minimize the attack surface
    rm -rf /usr/bin /usr/sbin && \
# check if executable works in this container
    /usr/local/bin/origintrail-parachain --version

USER origintrail-parachain

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/origintrail-parachain"]
