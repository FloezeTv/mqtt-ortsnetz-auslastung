FROM rust:1.99-alpine AS builder
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build
RUN cargo install --locked cargo-chef

FROM builder AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM builder AS build
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin mqtt-ortsnetz-auslastung

FROM scratch

COPY --from=build /build/target/x86_64-unknown-linux-musl/release/mqtt-ortsnetz-auslastung /mqtt-ortsnetz-auslastung

CMD [ "/mqtt-ortsnetz-auslastung" ]
