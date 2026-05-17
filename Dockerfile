FROM alpine:3.22 as certs
RUN apk add --no-cache ca-certificates

FROM ubuntu:focal as builder

ENV TZ=Europe/Paris

ADD . /src
WORKDIR /src

COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    apt-get clean && rm -rf /var/lib/apt/lists/* && \
    apt-get update || true && \
    apt-get install -y --no-install-recommends ca-certificates gnupg dirmngr && \
    apt-key adv --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys 3B4FE6ACC0B21F32 || true && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
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
        libpython3.8 \
        libssl-dev \
        pkg-config \
        python3 && \
    curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.88.0 -y && \
    . $HOME/.cargo/env && \
    cargo build --verbose --release && \
    cargo install --path . && \
    rm -rf /var/lib/apt/lists/*

FROM ubuntu:focal

COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
COPY --from=builder /root/.cargo/bin/transcript_worker /usr/bin
COPY --from=builder /src/ressources /ressources

RUN apt-get clean && rm -rf /var/lib/apt/lists/* && \
    apt-get update || true && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libavcodec58 \
        libavdevice58 \
        libavfilter7 \
        libavformat58 \
        libavresample4 \
        libavutil56 \
        libssl1.1 && \
    rm -rf /var/lib/apt/lists/*

ENV AMQP_QUEUE=job_transcript

CMD ["transcript_worker"]