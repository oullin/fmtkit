# The buildx SecretsUsedInArgOrEnv check false-positives on GIT_CONFIG_KEY_0
# ("KEY" heuristic); nothing here is a secret.
# check=skip=SecretsUsedInArgOrEnv

# Runtime image for the self-contained fmtkit binary.
#
# Not buildable from the repository root: goreleaser's dockers_v2 pipe supplies
# the build context, containing this file plus the prebuilt release binaries
# under <os>/<arch>/. scripts/test-docker-smoke.sh assembles the same context
# shape for local and PR builds.
#
# The base must be glibc: the embedded bun sidecar and the napi bindings are
# gnu builds, so musl (Alpine) cannot run them.
FROM debian:trixie-slim

ARG TARGETPLATFORM

# Keep in sync with the go directive in packages/go/go.mod.
ARG GO_VERSION=1.26.5

# git: the binary's own file discovery (gitfiles) shells out to it.
RUN apt-get update \
	&& apt-get install -y --no-install-recommends git ca-certificates curl \
	&& rm -rf /var/lib/apt/lists/*

# The Go formatting pass needs the go command at runtime: goimports runs
# in-process but resolves imports through `go list` (x/tools gocommand), and
# it errors hard without it. Bundling the toolchain also lets the automatic
# `go vet` pass run instead of being skipped.
RUN arch="${TARGETPLATFORM#linux/}" \
	&& curl -fsSL "https://go.dev/dl/go${GO_VERSION}.linux-${arch}.tar.gz" | tar -C /usr/local -xz \
	&& /usr/local/go/bin/go version

ENV PATH=/usr/local/go/bin:$PATH

COPY $TARGETPLATFORM/fmtkit /usr/local/bin/fmtkit

# Pre-extract the embedded TS toolchain so ephemeral containers do not pay the
# ~30MB extraction on every run. `fmtkit lint` in an empty git repo extracts
# (runtime.Resolve) and then returns before spawning the bun sidecar, so the
# warm-up also works under qemu emulation for the arm64 build.
RUN set -eu; \
	scratch="$(mktemp -d)"; \
	git init -q "$scratch"; \
	(cd "$scratch" && XDG_CACHE_HOME=/tmp/fmtkit-warm fmtkit lint); \
	mkdir -p /opt/fmtkit; \
	mv "/tmp/fmtkit-warm/fmtkit/$(ls /tmp/fmtkit-warm/fmtkit)" /opt/fmtkit/toolchain; \
	rm -rf /tmp/fmtkit-warm "$scratch"; \
	# The extractor creates the directory via MkdirTemp (0700); open it up so
	# `docker run --user` can read the toolchain too.
	chmod 0755 /opt/fmtkit/toolchain; \
	test -x /opt/fmtkit/toolchain/fmtkit-ts-sidecar; \
	test -f /opt/fmtkit/toolchain/.fmtkit-complete

# FMTKIT_SUPPORT_DIR short-circuits cache resolution entirely (read-only
# filesystems, any --user, no HOME needed), so it must be set only after the
# warm-up above has populated the directory. HOME=/tmp keeps anything else
# that wants a home directory happy under `docker run -u`. The GIT_CONFIG_*
# trio lets git treat bind-mounted trees owned by another uid as safe; the
# binary's git children inherit it.
ENV FMTKIT_SUPPORT_DIR=/opt/fmtkit/toolchain \
	HOME=/tmp \
	GIT_CONFIG_COUNT=1 \
	GIT_CONFIG_KEY_0=safe.directory \
	GIT_CONFIG_VALUE_0=*

WORKDIR /work
ENTRYPOINT ["/usr/local/bin/fmtkit"]
CMD ["help"]
