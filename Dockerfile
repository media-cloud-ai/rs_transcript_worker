FROM ubuntu:jammy as builder
ENV TZ=Europe/Paris

ADD . /src
WORKDIR /src

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        clang \
        gcc \
        llvm \
        pkg-config \
        python3 \
        libavcodec-dev \
        libavdevice-dev \
        libavfilter-dev \
        libavformat-dev \
        libavutil-dev \
        libclang1 \
        libssl-dev && \
    curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain 1.88.0 -y && \
    . $HOME/.cargo/env && \
    cargo build --verbose --release && \
    cargo install --path .

FROM ubuntu:jammy
COPY --from=builder /root/.cargo/bin/transcript_worker /usr/bin
COPY --from=builder /src/ressources /ressources

RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libavcodec59 \
        libavdevice59 \
        libavfilter8 \
        libavformat59 \
        libavutil57 \
        libssl3

ENV AMQP_QUEUE job_transcript
CMD transcript_worker
