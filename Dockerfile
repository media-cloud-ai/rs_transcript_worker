FROM alpine:3.22 as certs
RUN apk add --no-cache ca-certificates


FROM ubuntu:22.04 as builder

ENV TZ=Europe/Paris
ENV DEBIAN_FRONTEND=noninteractive
WORKDIR /src
COPY . .
COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    sed -i 's|http://archive.ubuntu.com/ubuntu|https://archive.ubuntu.com/ubuntu|g; \
            s|http://security.ubuntu.com/ubuntu|https://security.ubuntu.com/ubuntu|g' /etc/apt/sources.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        clang \
        curl \
        gcc \
        llvm \
        libavcodec-dev \
        libavdevice-dev \
        libavfilter-dev \
        libavformat-dev \
        libavresample-dev \
        libavutil-dev \
        libclang1 \
        libssl-dev \
        pkg-config \
        python3 && \
    update-ca-certificates && \
    curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain stable -y && \
    . "$HOME/.cargo/env" && \
    cargo build --release && \
    cargo install --path . && \
    rm -rf /var/lib/apt/lists/*


FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
COPY --from=builder /root/.cargo/bin/transcript_worker /usr/bin/
COPY --from=builder /src/ressources /ressources

RUN sed -i 's|http://archive.ubuntu.com/ubuntu|https://archive.ubuntu.com/ubuntu|g; \
            s|http://security.ubuntu.com/ubuntu|https://security.ubuntu.com/ubuntu|g' /etc/apt/sources.list && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libavcodec59 \
        libavdevice59 \
        libavfilter8 \
        libavformat59 \
        libavutil57 \
        libswresample4 \
        libssl3 && \
    rm -rf /var/lib/apt/lists/*

ENV AMQP_QUEUE=job_transcript

CMD ["transcript_worker"]