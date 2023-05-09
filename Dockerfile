# Use a docker build container with musl-gcc installed
FROM ekidd/rust-musl-builder:latest AS build

# create an empty project
RUN cargo new lichen

# prefetch and cache dependencies
WORKDIR lichen
COPY Cargo.toml Cargo.lock .
RUN cargo fetch 

# copy source and build for Alpine
COPY ./src ./src/
RUN cargo build --target x86_64-unknown-linux-musl  --release

# Use a slim container with just our app to publish
FROM alpine:latest AS app

COPY --from=build /home/rust/src/lichen/target/x86_64-unknown-linux-musl/release/lichen /

# Command
ENTRYPOINT ["/lichen"]

# default argument
CMD ["--help"]

