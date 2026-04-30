# Build the static musl binaries first:
#   ./scripts/musl-build.sh --release
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

ARG BINARY
ENV BINARY=${BINARY}

COPY target-main/x86_64-unknown-linux-musl/release/${BINARY} /usr/local/bin/${BINARY}
RUN chmod +x /usr/local/bin/${BINARY}

ENTRYPOINT ["/bin/sh", "-c", "exec /usr/local/bin/$BINARY \"$@\"", "--"]
CMD ["serve"]
