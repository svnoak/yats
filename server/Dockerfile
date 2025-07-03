FROM rust:1-slim-buster AS builder

WORKDIR /app

COPY ./server/ ./

RUN echo "fn main() {}" > src/main.rs

RUN cargo build --release

RUN rm -f src/main.rs
RUN rm -f target/release/deps/yats_server*

COPY ./server/src ./src

RUN cargo build --release

FROM debian:buster-slim

RUN groupadd -r appuser && useradd -r -g appuser appuser
USER appuser

COPY --from=builder /app/target/release/yats-server /usr/local/bin/

EXPOSE 3000

CMD ["/usr/local/bin/yats-server"]
