FROM rust:1.61.0-buster AS builder
WORKDIR /usr/src/

RUN USER=root cargo new --lib identity-cloud-agent
WORKDIR /usr/src/identity-cloud-agent
COPY Cargo.toml ./
RUN echo "fn main() {}" > src/bin.rs
RUN cargo build --release
RUN rm src/*.rs
COPY src ./src
RUN touch src/lib.rs
RUN touch src/bin.rs
RUN cargo build --release
RUN ls target/release/

FROM rust:1.61.0-slim-buster

COPY --from=builder /usr/src/identity-cloud-agent/target/release/identity_cloud_agent_bin /bin
USER 1000
COPY Rocket.toml ./Rocket.toml
CMD [ "/bin/identity_cloud_agent_bin" ]
