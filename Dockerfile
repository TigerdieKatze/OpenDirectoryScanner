FROM rust:1.86-slim AS builder
 
WORKDIR /app
 
# Install dependencies
RUN apt-get update && \
    apt-get install -y curl pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*
 
# Create a new empty project
RUN USER=root cargo new --bin opendirectoryscanner
WORKDIR /app/opendirectoryscanner
 
# Copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml
 
# Cache dependencies
RUN mkdir -p resources
RUN curl -L -o resources/model.onnx https://github.com/Fyko/nsfw/releases/latest/download/model.onnx
 
# Build dependencies - this is the caching Docker layer
RUN cargo build --release
RUN rm src/*.rs
 
# Copy actual source code
COPY ./src ./src
COPY ./resources ./resources
 
# Build the application
RUN touch src/main.rs
RUN cargo build --release
 
# Create a smaller runtime image
FROM debian:stable-slim
 
WORKDIR /app
 
# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates libssl1.1 && \
    rm -rf /var/lib/apt/lists/*
 
# Copy the binary from the builder stage
COPY --from=builder /app/opendirectoryscanner/target/release/opendirectoryscanner /app/opendirectoryscanner
COPY --from=builder /app/opendirectoryscanner/resources /app/resources
 
# Set environment variables
ENV RUST_LOG=info
 
# Set the entrypoint
ENTRYPOINT ["/app/opendirectoryscanner"]
 
# Default command (can be overridden)
CMD ["--help"]