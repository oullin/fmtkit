# Runtime image for fmtkit and its Go helper.
#
# Not buildable from the repository root: the release workflow supplies a build
# context holding this file plus the prebuilt binaries under <os>/<arch>/.
# scripts/test-docker-smoke.sh assembles the same context shape locally.
FROM debian:trixie-slim

ARG TARGETPLATFORM

# Keep in sync with the go directive in go/helper/go.mod.
ARG GO_VERSION=1.27.1

RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates curl \
	&& rm -rf /var/lib/apt/lists/*

# The go command is only needed for `go vet` and for goimports' opt-in import
# resolution; formatting itself runs in the helper without it.
RUN arch="${TARGETPLATFORM#linux/}" \
	&& curl -fsSL "https://go.dev/dl/go${GO_VERSION}.linux-${arch}.tar.gz" | tar -C /usr/local -xz \
	&& /usr/local/go/bin/go version

COPY $TARGETPLATFORM/fmtkit $TARGETPLATFORM/fmtkit-go-helper /usr/local/bin/

# HOME=/tmp gives `docker run -u` a writable cache directory and a Go build
# cache for vet.
ENV PATH=/usr/local/go/bin:$PATH \
	HOME=/tmp

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/fmtkit"]
CMD ["--help"]
