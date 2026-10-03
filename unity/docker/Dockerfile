FROM unityci/editor:ubuntu-2022.3.22f1-base-3.2.2

ARG VRC_GET_VERSION=1.9.2

RUN apt-get update && apt-get install -y --no-install-recommends \
        ca-certificates curl python3 \
        libgl1 libglx-mesa0 libgl1-mesa-dri libvulkan1 \
    && rm -rf /var/lib/apt/lists/* \
    && curl -L --fail --retry 3 --retry-delay 2 -o /usr/local/bin/vrc-get \
        "https://github.com/vrc-get/vrc-get/releases/download/v${VRC_GET_VERSION}/x86_64-unknown-linux-musl-vrc-get" \
    && chmod +x /usr/local/bin/vrc-get \
    && mkdir -p /tmp/.X11-unix && chmod 1777 /tmp/.X11-unix

WORKDIR /project
