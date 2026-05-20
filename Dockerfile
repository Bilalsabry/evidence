# syntax=docker/dockerfile:1.7

# Reproducibility image for the evidence-eval artifact (ACL artifact
# evaluation). One image, one shell, every paper number reproducible.
#
# Pinned bases:
#   - Debian 12 (bookworm) slim — small, common, long-lived.
#   - Rust 1.85.0 — matches workspace `rust-version = "1.85"`
#     (see Cargo.toml) and the `rust-toolchain.toml` stable channel.
#
# What this image contains:
#   - Rust stable 1.85.0 + cargo + rustfmt + clippy.
#   - libssl + pkg-config + ca-certificates + git + curl — the only
#     system deps the harness needs (rusqlite is `bundled`).
#
# What this image deliberately does NOT contain:
#   - PDFium: auto-downloaded by `pdfium-auto` on first PDF call.
#   - DeBERTa / distilbert ONNX models: pulled by the harness from
#     HuggingFace on the first `--real-nli` / `compare` / `latency`
#     call (~700 MB total, one-time, then cached).
#   - The DailyMed corpus: re-fetched via
#     `evidence-eval fetch dailymed --limit 50 --output /tmp/corpus`.
#
# Build:
#   docker build -t evidence-eval:repro .
#
# Run (mounts the repo so reviewer artifacts land on the host):
#   docker run --rm -it -v "$PWD":/workspace -w /workspace \
#       evidence-eval:repro

FROM debian:bookworm-slim

ENV DEBIAN_FRONTEND=noninteractive \
    RUST_VERSION=1.85.0 \
    RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

# System dependencies. Pinned to bookworm's apt index; no version
# floats. `--no-install-recommends` keeps the image small.
# hadolint ignore=DL3008
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        git \
        libssl-dev \
        pkg-config \
        build-essential \
    && rm -rf /var/lib/apt/lists/*

# Install rustup + the exact stable toolchain the workspace pins,
# plus the components `rust-toolchain.toml` asks for.
# hadolint ignore=DL4006
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y \
            --no-modify-path \
            --profile minimal \
            --default-toolchain "${RUST_VERSION}" \
            --component rustfmt clippy \
    && chmod -R a+w "${RUSTUP_HOME}" "${CARGO_HOME}" \
    && rustc --version \
    && cargo --version

WORKDIR /workspace

# Default shell — reviewers `docker run -it ... bash` and drive the
# harness interactively per docs/paper/REPRODUCIBILITY.md.
CMD ["bash"]
