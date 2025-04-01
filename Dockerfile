# Use the official Rust image as the base image
FROM rust:1.80 as builder

# Set the working directory inside the container
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock ./

# Copy the source code
COPY src ./src

# Install dependencies and build the application in release mode
RUN cargo build --release

# Use a minimal base image for the final container
FROM debian:buster-slim

# Set the working directory inside the container
WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /app/target/release/my-api-service .

# Expose the port your application runs on
EXPOSE 5000

# Set the command to run the application
CMD ["./my-api-service"]