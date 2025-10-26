# syntax=docker/dockerfile:1.7

###############################
# Stage 1 - Rust backend build
###############################
FROM rust:1.86-slim AS backend-builder

ARG APP_NAME=rcs
WORKDIR /app

# install build dependencies required by Diesel
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        libpq-dev=15.14-0+deb12u1 \
        libssl-dev=3.0.17-1~deb12u3 \
        pkg-config=1.8.1-1 \
        ca-certificates=20230311+deb12u1 \
    && rm -rf /var/lib/apt/lists/*

# leverage Docker layer caching by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY diesel.toml ./
COPY build.rs ./
COPY rust-toolchain.toml ./

# copy source and migrations
COPY src ./src
COPY migrations ./migrations
COPY benches ./benches

# compile the backend in release mode
RUN cargo build --release --locked

#########################################
# Stage 2 - Runtime image (distroless-ish)
#########################################
FROM debian:bookworm-slim AS runtime

ARG APP_NAME=rcs
ENV APP_HOME=/app \
    APP_USER=appuser

# install runtime dependencies and create non-root user
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        curl=7.88.1-10+deb12u14 \
        libpq5=15.14-0+deb12u1 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir ${APP_HOME} ${APP_USER}

WORKDIR ${APP_HOME}

# copy backend artifacts
COPY --from=backend-builder /app/target/release/${APP_NAME} ${APP_HOME}/backend
COPY --from=backend-builder /app/migrations ${APP_HOME}/migrations
COPY --from=backend-builder /app/diesel.toml ${APP_HOME}/diesel.toml

# ancillary scripts
COPY wait-for-it.sh ${APP_HOME}/wait-for-it.sh
RUN chmod +x ${APP_HOME}/wait-for-it.sh \
    && chown -R ${APP_USER}:${APP_USER} ${APP_HOME}

EXPOSE 8000

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8000/health || exit 1

USER ${APP_USER}
ENV RUST_LOG=info

CMD ["/app/backend"]
