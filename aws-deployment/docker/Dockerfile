# Simple runtime-only image
FROM ubuntu:22.04

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1001 qed

WORKDIR /app

# Copy the compiled binaries from local target directory
# Make sure to run 'cargo build --release' before building the image
COPY target/release/qed_rollup_cli /usr/local/bin/
COPY target/release/qed_user_prover /usr/local/bin/

# Make binaries executable
RUN chmod +x /usr/local/bin/qed_rollup_cli /usr/local/bin/qed_user_prover

# Create necessary directories
RUN mkdir -p /data && chown -R qed:qed /data

# Switch to non-root user
USER qed

# Volume for persistent data
VOLUME ["/data"]

# Set entrypoint to qed_rollup_cli
ENTRYPOINT ["qed_rollup_cli"]

# Default command (can be overridden by ECS task definitions)
CMD ["--help"]