FROM rust:1.99-bookworm AS build
WORKDIR /src
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
RUN cargo build --release --locked

FROM python:3.12-slim-bookworm AS runtime-lib
RUN pip install --no-cache-dir onnxruntime==1.23.2
RUN mkdir /ort && cp /usr/local/lib/python3.12/site-packages/onnxruntime/capi/libonnxruntime.so.1.23.2 /ort/

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates libgomp1 curl && rm -rf /var/lib/apt/lists/* && useradd -m -u 10001 qc
COPY --from=build /src/target/release/manufacturing-qc /usr/local/bin/manufacturing-qc
COPY --from=runtime-lib /ort /opt/ort
ENV ORT_DYLIB_PATH=/opt/ort/libonnxruntime.so.1.23.2 BIND_ADDRESS=0.0.0.0 PORT=5050
RUN mkdir /storage /artifacts && chown qc:qc /storage /artifacts
USER qc
EXPOSE 5050
HEALTHCHECK --interval=30s --timeout=10s CMD curl -fsS http://localhost:5050/ready || exit 1
ENTRYPOINT ["manufacturing-qc"]
