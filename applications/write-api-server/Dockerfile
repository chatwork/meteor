# syntax=docker/dockerfile:1.5
FROM --platform=$BUILDPLATFORM messense/rust-musl-cross:${TARGETARCH}-musl as builder

ARG NAME=cqrs-es-example-write-api-server
ARG TARGETARCH

RUN if [ $TARGETARCH = "amd64" ]; then \
      echo "x86_64" > /arch; \
    elif [ $TARGETARCH = "arm64" ]; then \
      echo "aarch64" > /arch; \
    else \
      echo "Unsupported platform: $TARGETARCH"; \
      exit 1; \
    fi

COPY . /app
WORKDIR /app

ENV CARGO_BUILD_TARGET_DIR /tmp/target

RUN CARGO_BUILD_INCREMENTAL=true cargo build -p ${NAME} --release --target $(cat /arch)-unknown-linux-musl \
    && mkdir -p /home/rust/src/target/release \
    && cp /tmp/target/$(cat /arch)-unknown-linux-musl/release/${NAME} /home/rust/src/target/release/app

FROM --platform=$TARGETPLATFORM alpine
RUN apk --no-cache add ca-certificates
RUN mkdir -p /config
COPY --from=builder /home/rust/src/target/release/app /app
CMD [ "/app" ]