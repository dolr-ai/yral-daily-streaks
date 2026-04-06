# 2. Final Image
FROM scratch
WORKDIR /app

COPY ./target/x86_64-unknown-linux-musl/release/yral-or-not .
COPY ./config.toml .

ENV RUST_LOG="debug"
ENV BIND_ADDRESS="0.0.0.0:3000"
ENV APP_ENV="development"
EXPOSE 3000

CMD ["./yral-daily-streaks"]