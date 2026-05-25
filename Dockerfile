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

RUN CGO_ENABLED=0 GOOS=${TARGETOS} GOARCH=${TARGETARCH} \
	go build -trimpath -ldflags="-s -w" -o /out/gofmt cmd/gofmt

RUN CGO_ENABLED=0 GOOS=${TARGETOS} GOARCH=${TARGETARCH} GOPATH=/tmp/go \
	go install -trimpath -ldflags="-s -w" golang.org/x/tools/cmd/goimports@v0.43.0 && \
	find /tmp/go/bin -name goimports -exec cp {} /out/goimports \;

FROM golang:1.25-alpine AS gosdk

FROM node:25.8.2-alpine AS formatter

RUN apk add --no-cache bash git

WORKDIR /work

COPY --from=gosdk /usr/local/go /usr/local/go
ENV PATH="/usr/local/go/bin:${PATH}" \
	GOCACHE="/work/storage/.cache/go-build" \
	GOPATH="/work/storage/.cache/gopath" \
	GOMODCACHE="/work/storage/.cache/gopath/pkg/mod" \
	TURBO_CACHE_DIR="/work/storage/.cache/turbo"

COPY --from=builder /out/go-fmt /usr/local/bin/go-fmt
COPY --from=builder /out/gofmt /usr/local/bin/gofmt
COPY --from=builder /out/goimports /usr/local/bin/goimports

WORKDIR /opt/go-fmt/support
COPY packages/devx/package.json /opt/go-fmt/support/package.json
RUN node -e 'const p = require("./package.json"); const deps = ["oxc-parser", "oxfmt", "tsx"]; console.log(deps.map((name) => `${name}@${p.devDependencies[name]}`).join(" "));' \
	| xargs npm install --no-save

COPY packages/devx/scripts/blank-lines.ts /opt/go-fmt/support/blank-lines.ts
COPY scripts/format-ts.sh /usr/local/bin/format-ts
COPY scripts/formatter-entrypoint.sh /usr/local/bin/formatter-entrypoint

WORKDIR /work

ENTRYPOINT ["/usr/local/bin/formatter-entrypoint"]
