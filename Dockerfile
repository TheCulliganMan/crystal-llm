FROM rust:1.94-bookworm AS build
WORKDIR /source

RUN rustup target add wasm32-unknown-unknown \
    && cargo install wasm-bindgen-cli --version 0.2.126 --locked

WORKDIR /source/rust
COPY rust/Cargo.toml rust/Cargo.lock ./
COPY rust/.cargo ./.cargo
COPY rust/crates ./crates
ARG CARGO_BUILD_JOBS=2
# Keep dependency and target caches across source edits. Run from the workspace
# so Cargo reads its WASM target configuration.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/source/rust/target,sharing=locked \
    cargo build --locked --release --package crystal-web-server --bin crystal-web-server \
    && cargo build --locked --profile web-release --package crystal-bevy \
        --bin crystal-bevy --features fullscreen-scaling --target wasm32-unknown-unknown \
    && mkdir -p /out/web \
    && cp target/release/crystal-web-server /out/crystal-web-server \
    && wasm-bindgen --target web --out-dir /out/web --out-name crystal-bevy \
        target/wasm32-unknown-unknown/web-release/crystal-bevy.wasm \
    && gzip -9 -k /out/web/crystal-bevy_bg.wasm

# Page, audio, and pack updates do not invalidate the Rust compilation layer.
COPY rust/web-client /source/web-client
COPY content-packs/core-modular.browser.crystalpack /out/web/core-modular.browser.crystalpack
RUN cp /source/web-client/index.html /source/web-client/browser-session.js /source/web-client/webmcp.js /out/web/ \
    && cp -R /source/web-client/audio-runtime /out/web/audio-runtime \
    && gzip -9 -k /out/web/core-modular.browser.crystalpack

FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

COPY --from=build /out/crystal-web-server /usr/local/bin/crystal-web-server
COPY --from=build --chown=65532:65532 /out/web /srv/crystal/web
COPY --chown=65532:65532 docker/runtime-data /srv/crystal/data

ENV CRYSTAL_HOST=0.0.0.0 \
    CRYSTAL_PORT=8080 \
    CRYSTAL_WEB_ROOT=/srv/crystal/web \
    CRYSTAL_PACK_DIR=/srv/crystal/packs

EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/usr/local/bin/crystal-web-server"]
