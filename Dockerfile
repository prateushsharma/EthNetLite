# Use the official Rust image as the base image for building
FROM rust:latest as builder

# Install protobuf compiler
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY src ./src
COPY proto ./proto
COPY build.rs ./

# Build the application in release mode
RUN cargo build --release

# Use a minimal base image for the final stage
FROM debian:bookworm-slim

# Install necessary runtime dependencies (if any)
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/EthNetLite /usr/local/bin/EthNetLite

# Expose the default ports (adjust as needed)
EXPOSE 9001 10001

# Set the default command to run the application
# Note: You may need to pass arguments for port and bootstrap
CMD ["EthNetLite", "9001"]