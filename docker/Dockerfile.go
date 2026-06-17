FROM --platform=$BUILDPLATFORM golang:1.26-bookworm AS builder

ARG TARGETOS
ARG TARGETARCH

WORKDIR /src

COPY go.work /src/go.work
COPY packages/formatter/go.mod packages/formatter/go.sum /src/packages/formatter/
COPY packages/driver/go.mod packages/driver/go.sum /src/packages/driver/
COPY packages/vet/go.mod /src/packages/vet/

RUN go -C /src/packages/formatter mod download
RUN go -C /src/packages/driver mod download

RUN mkdir -p /out && \
	CGO_ENABLED=0 GOOS=${TARGETOS} GOARCH=${TARGETARCH} GOPATH=/tmp/go \
	go install -trimpath -ldflags="-s -w" golang.org/x/tools/cmd/goimports@v0.43.0 && \
	find /tmp/go/bin -name goimports -exec cp {} /out/goimports \;

COPY . .

RUN bash -lc 'source /src/scripts/env.sh && assert_no_legacy_artifacts'

ARG VERSION=dev

RUN CGO_ENABLED=0 GOOS=${TARGETOS} GOARCH=${TARGETARCH} \
	go -C /src/packages/driver build -trimpath -ldflags="-s -w -X main.version=${VERSION}" -o /out/fmt-go ./cmd/fmt-go

FROM golang:1.26-alpine

RUN apk add --no-cache bash git

# Strip Go SDK parts neither `goimports` nor `go vet` uses at runtime: Go's own
# test suite, API compatibility data, docs, std-library test fixtures, and tool
# binaries only used by `go fix`/`go test -cover`/PGO.
RUN cd /usr/local/go && \
	rm -rf test api doc misc && \
	find src -type d -name testdata -prune -exec rm -rf {} + && \
	find src -type f -name '*_test.go' -delete && \
	rm -f pkg/tool/*/fix pkg/tool/*/cover pkg/tool/*/preprofile

WORKDIR /work

ENV GOCACHE="/work/storage/.cache/go-build" \
	GOPATH="/work/storage/.cache/gopath" \
	GOMODCACHE="/work/storage/.cache/gopath/pkg/mod"

COPY --from=builder /out/fmt-go /usr/local/bin/fmt-go
COPY --from=builder /out/goimports /usr/local/bin/goimports

ENTRYPOINT ["/usr/local/bin/fmt-go"]
