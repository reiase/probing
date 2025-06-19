# Build probe
FROM rust:slim-bookworm AS builder

RUN apt update && apt install -y curl xz-utils && apt clean all

WORKDIR /workspace

COPY Cargo.toml Cargo.toml
COPY Cargo.lock Cargo.lock
COPY app/ app/
COPY probing/ probing/
COPY src/ src/

RUN curl -O https://ziglang.org/download/0.12.0/zig-linux-x86_64-0.12.0.tar.xz && \
    tar -xf zig-linux-x86_64-0.12.0.tar.xz && \
    mv zig-linux-x86_64-0.12.0 zig && \
    mv zig /usr/local/

RUN cargo install --locked cargo-zigbuild
RUN CARGO_ZIGBUILD_ZIG_PATH=/usr/local/zig/zig cargo zigbuild --target x86_64-unknown-linux-gnu -r
RUN CARGO_ZIGBUILD_ZIG_PATH=/usr/local/zig/zig cargo zigbuild --target x86_64-unknown-linux-gnu -r --package probing-cli


#FROM 10.200.53.208/public/xmegatron:v1.2.0
FROM 10.200.53.208/cd/nh-llm-train:v0.1.0-nhtorch0.1.1v-nhllmops0.5.0v-rc1-zccl-1.4.2-2025061801
ARG TARGET_DIR=target/release/
RUN [ -d "$TARGET_DIR" ] || mkdir -p "$TARGET_DIR"
COPY --from=builder /workspace/target/x86_64-unknown-linux-gnu/release/libprobing.so $TARGET_DIR
COPY --from=builder /workspace/target/x86_64-unknown-linux-gnu/release/probing $TARGET_DIR
COPY make_wheel.py make_wheel.py
COPY Cargo.toml Cargo.toml
COPY README.md README.md
COPY python/ python/
COPY examples/ examples/

RUN /root/miniconda/envs/python310_torch25_cuda/bin/pip install toml
RUN /root/miniconda/envs/python310_torch25_cuda/bin/python make_wheel.py
RUN /root/miniconda/envs/python310_torch25_cuda/bin/pip install dist/*
ENV PROBING_PORT=80
ENV PROBING=init:/home/examples/job_tracker.py+1
CMD ["bash"]
