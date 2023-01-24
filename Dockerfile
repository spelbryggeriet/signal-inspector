###############################################################################
# Backend building stage                                                      #
###############################################################################
FROM rust:1.66 as build-backend

# Install dependencies
RUN apt update
RUN apt install -y gcc-aarch64-linux-gnu

# Add target
RUN rustup target add aarch64-unknown-linux-gnu

# Create a new empty project for the backend
RUN USER=root cargo new --bin backend
WORKDIR /backend

# Create cargo config for setting the linker command
RUN mkdir ./.cargo
RUN echo "[target.aarch64-unknown-linux-gnu]\nlinker = \"aarch64-linux-gnu-gcc\"" > ./.cargo/config

# Copy our manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./backend/Cargo.toml ./Cargo.toml

# Build only the dependencies to cache them
RUN cargo build --release --target aarch64-unknown-linux-gnu
RUN rm src/*.rs

# Copy the source code
COPY ./backend/src ./src

# Build for release
RUN rm ./target/aarch64-unknown-linux-gnu/release/deps/signal_inspector_backend*
RUN cargo build --release --target aarch64-unknown-linux-gnu

###############################################################################
# Frontend building stage                                                     #
###############################################################################
FROM rust:1.66 as build-frontend

# Add target
RUN rustup target add wasm32-unknown-unknown

# Install trunk command
RUN cargo install trunk@0.16.0

# Create a new empty project for the backend
RUN USER=root cargo new --bin frontend
WORKDIR /frontend

# Copy our manifests and index file
COPY ./Cargo.lock ./Cargo.lock
COPY ./frontend/Cargo.toml ./Cargo.toml
COPY ./frontend/index.html ./index.html

# Build only the dependencies to cache them
RUN cargo build --release --target wasm32-unknown-unknown
RUN rm src/*.rs

# Copy the source code
COPY ./frontend/src ./src

# Build for release
RUN rm ./target/wasm32-unknown-unknown/release/deps/signal_inspector_frontend*
RUN trunk build --release

###############################################################################
# Final stage                                                                 #
###############################################################################
FROM --platform=linux/arm64/v8 debian:buster-slim

# Copy server config
COPY ./backend/Rocket.toml /serve/Rocket.toml

# Copy from the previous builds
COPY --from=build-backend /backend/target/aarch64-unknown-linux-gnu/release/signal-inspector-backend /serve/signal-inspector-backend
COPY --from=build-frontend /frontend/dist /serve/static

# Run the binary
ENV SIGNAL_INSPECTOR_STATIC_DIR=/serve/static
ENV ROCKET_CONFIG=/serve/Rocket.toml
CMD ["/serve/signal-inspector-backend"]
