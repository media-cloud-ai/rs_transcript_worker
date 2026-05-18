FROM alpine:3.22 as certs
RUN apk add --no-cache ca-certificates

FROM ubuntu:focal as builder

ENV TZ=Europe/Paris

ADD . /src
WORKDIR /src

COPY --from=certs /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone && \
    apt-get clean && rm -rf /var/lib/apt/lists/* && \
    mkdir -p /var/lib/apt/lists/partial && \
    echo 'Binary::apt::APT::Keep-Downloaded-Packages "false";' > /etc/apt/apt.conf.d/no-cache && \
    apt-get -o Acquire::AllowInsecureRepositories=true \
            -o Acquire::AllowDowngradeToInsecureRepositories=true \
            update || true && \
    apt-get install -y --no-install-recommends --allow-unauthenticated \
        ca-certificates ubuntu-keyring && \
    rm -rf /var/cache/apt/archives/*.deb && \
    rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/* && \
    apt-get -o Acquire::AllowInsecureRepositories=true update || true && \
    apt-get install -y --no-install-recommends --allow-unauthenticated \
    gnupg dirmngr \
    clang curl gcc llvm \
    libavcodec-dev libavdevice-dev libavfilter-dev \
    libavformat-dev libavresample-dev libavutil-dev \
    libclang1 libpython3.8 libssl-dev pkg-config python3 && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/* /var/cache/apt/archives/* && \
    curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain 1.88.0 -y && \
    . $HOME/.cargo/env && \
    cargo update -p time && \
    cargo build --release && \
    cargo install --path . && \
    cargo clean && \
    rm -rf target && \
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