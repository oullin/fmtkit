FROM --platform=$BUILDPLATFORM golang:1.25-bookworm AS builder

ARG TARGETOS
ARG TARGETARCH
ARG VERSION=dev

WORKDIR /src

COPY go.work /src/go.work
COPY packages/formatter/go.mod packages/formatter/go.sum /src/packages/formatter/
COPY packages/driver/go.mod packages/driver/go.sum /src/packages/driver/
COPY packages/vet/go.mod /src/packages/vet/

RUN go -C /src/packages/formatter mod download
RUN go -C /src/packages/driver mod download

COPY . .

RUN bash -lc 'source /src/scripts/env.sh && assert_no_legacy_artifacts'

RUN CGO_ENABLED=0 GOOS=${TARGETOS} GOARCH=${TARGETARCH} \
	go -C /src/packages/driver build -trimpath -ldflags="-s -w -X main.version=${VERSION}" -o /out/go-fmt ./cmd/fmt

RUN CGO_ENABLED=0 GOOS=${TARGETOS} GOARCH=${TARGETARCH} GOPATH=/tmp/go \
	go install -trimpath -ldflags="-s -w" golang.org/x/tools/cmd/goimports@v0.43.0 && \
	find /tmp/go/bin -name goimports -exec cp {} /out/goimports \;

FROM golang:1.25-alpine

RUN apk add --no-cache bash git

WORKDIR /work

ENV GOCACHE="/work/storage/.cache/go-build" \
	GOPATH="/work/storage/.cache/gopath" \
	GOMODCACHE="/work/storage/.cache/gopath/pkg/mod"

COPY --from=builder /out/go-fmt /usr/local/bin/go-fmt
COPY --from=builder /out/goimports /usr/local/bin/goimports

ENTRYPOINT ["/usr/local/bin/go-fmt"]
