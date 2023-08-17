FROM rust:latest AS build
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates
COPY --from=build /app/target/release/rusty_reader .
EXPOSE 3000
CMD ["./rusty_reader"]