# Build the static musl binaries first:
#   ./scripts/release-build.sh
FROM alpine:3.21

RUN apk add --no-cache ca-certificates

ARG BINARY
ENV BINARY=${BINARY}

COPY target-release/x86_64-unknown-linux-musl/release/${BINARY} /usr/local/bin/${BINARY}
RUN chmod +x /usr/local/bin/${BINARY}

ENTRYPOINT ["/bin/sh", "-c", "exec /usr/local/bin/$BINARY \"$@\"", "--"]
CMD ["serve"]
