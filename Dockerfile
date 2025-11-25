FROM ubuntu:noble AS builder
ENV TZ=Europe/Paris

COPY . /src
WORKDIR /src

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    apt-get update && \
    apt-get install -y \
    clang \
    curl \
    gcc \
    llvm \
    libavcodec-dev \
    libavdevice-dev \
    libavfilter-dev \
    libavformat-dev \
    libswresample-dev \
    libavutil-dev \
    libclang1 \
    libpython3.8 \
    libssl-dev \
    pkg-config \
    python3 \
    && \
    curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain 1.88.0 -y && \
    . $HOME/.cargo/env && \
    cargo build --verbose --release && \
    cargo install --path .

FROM ubuntu:noble
COPY --from=builder /root/.cargo/bin/transcript_worker /usr/bin
COPY --from=builder /src/ressources /ressources

RUN apt update && \
    apt install -y \
    ca-certificates \
    libavcodec60 \
    libavdevice60 \
    libavfilter-extra9 \
    libavformat60 \
    libswresample4 \
    libavutil58 \
    libssl3

ENV AMQP_QUEUE=job_transcript
CMD ["transcript_worker"]
