# Build Stage
FROM rust:1.55-buster AS build
USER 0:0
WORKDIR /home/rust

RUN USER=root cargo new --bin vortex
WORKDIR /home/rust/vortex

COPY Cargo.toml Cargo.lock ./
RUN apt-get update && \
    apt-get -y install python3 python3-pip \
    build-essential g++ gcc clang cmake make \
    ninja-build sudo bash curl
RUN curl -fsSL https://deb.nodesource.com/setup_12.x | sudo -E bash - && \
    apt-get update && \
    apt-get -y install nodejs

RUN cargo build --locked --release

RUN rm src/*.rs target/release/deps/vortex*
COPY src ./src
RUN cargo install --locked --path .

# Bundle Stage
FROM debian:bullseye

COPY --from=build /usr/local/cargo/bin/vortex ./vortex

EXPOSE 8080
ENV HTTP_HOST 0.0.0.0:8080

CMD ["./vortex"]
