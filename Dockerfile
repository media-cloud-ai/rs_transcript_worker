FROM ubuntu:focal as builder
ENV TZ=Europe/Paris

ADD . /src
WORKDIR /src

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    sed -i 's|http://archive.ubuntu.com/ubuntu|https://archive.ubuntu.com/ubuntu|g; s|http://security.ubuntu.com/ubuntu|https://security.ubuntu.com/ubuntu|g' /etc/apt/sources.list && \
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
        python3 \
        && \
    curl https://sh.rustup.rs -sSf | \
    sh -s -- --default-toolchain 1.88.0 -y && \
    . $HOME/.cargo/env && \
    cargo build --verbose --release && \
    cargo install --path . && \
    rm -rf /var/lib/apt/lists/*

FROM ubuntu:focal
COPY --from=builder /root/.cargo/bin/transcript_worker /usr/bin
COPY --from=builder /src/ressources /ressources

RUN sed -i 's|http://archive.ubuntu.com/ubuntu|https://archive.ubuntu.com/ubuntu|g; s|http://security.ubuntu.com/ubuntu|https://security.ubuntu.com/ubuntu|g' /etc/apt/sources.list && \
    apt-get update && \
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

ENV AMQP_QUEUE job_transcript
CMD transcript_worker
