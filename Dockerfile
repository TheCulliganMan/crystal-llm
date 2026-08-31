FROM rust:1.94-bookworm AS build
WORKDIR /source

RUN rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.126 --locked

COPY rust ./rust
COPY content-packs/core-modular.browser.crystalpack ./content-packs/core-modular.browser.crystalpack
RUN cargo build \
    --locked \
    --release \
    --manifest-path rust/Cargo.toml \
    --package crystal-web-server \
    --bin crystal-web-server \
    && cargo build \
        --locked \
        --profile web-release \
        --manifest-path rust/Cargo.toml \
        --package crystal-bevy \
        --bin crystal-bevy \
        --target wasm32-unknown-unknown \
    && mkdir -p rust/web-dist \
    && wasm-bindgen \
        --target web \
        --out-dir rust/web-dist \
        --out-name crystal-bevy \
        rust/target/wasm32-unknown-unknown/web-release/crystal-bevy.wasm \
    && cp rust/web-client/index.html rust/web-dist/index.html \
    && cp -R rust/web-client/audio-runtime rust/web-dist/audio-runtime \
    && cp content-packs/core-modular.browser.crystalpack rust/web-dist/ \
    && gzip -9 -k rust/web-dist/crystal-bevy_bg.wasm \
    && gzip -9 -k rust/web-dist/core-modular.browser.crystalpack

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

COPY --from=build /source/rust/target/release/crystal-web-server /usr/local/bin/crystal-web-server
COPY --from=build --chown=65532:65532 /source/rust/web-dist /srv/crystal/web
COPY --chown=65532:65532 docker/runtime-data /srv/crystal/data

ENV CRYSTAL_HOST=0.0.0.0 \
    CRYSTAL_PORT=8080 \
    CRYSTAL_WEB_ROOT=/srv/crystal/web \
    CRYSTAL_PACK_DIR=/srv/crystal/packs

EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/crystal-web-server"]
