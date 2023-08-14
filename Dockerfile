FROM rust:latest AS build
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:latest
WORKDIR /app
COPY --from=build /app/target/release/put_name_here_dan .
EXPOSE 3000
CMD ["./put_name_here_dan"]