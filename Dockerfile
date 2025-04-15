FROM rust:1.86-slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && \
    apt-get install -y curl pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock

# Download model if not present
RUN mkdir -p resources && \
    if [ ! -f resources/model.onnx ]; then \
      curl -L -o resources/model.onnx https://github.com/Fyko/nsfw/releases/latest/download/model.onnx; \
    fi

# Copy source code
COPY src src
COPY resources resources

# Build the application
RUN cargo build --release

# Create a smaller runtime image
FROM debian:stable-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl1.1 && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary and resources from the builder stage
COPY --from=builder /app/target/release/opendirectoryscanner /app/opendirectoryscanner
COPY --from=builder /app/resources /app/resources

ENV RUST_LOG=info

ENTRYPOINT ["/app/opendirectoryscanner"]
CMD ["--help"]