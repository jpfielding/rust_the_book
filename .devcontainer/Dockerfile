FROM rockylinux/rockylinux:10 AS base

ARG REPO_NAME=rust_the_book
ARG USERNAME=krusty
ARG GROUPNAME=${USERNAME}
ARG USER_UID=1000
ARG USER_GID=1000

# Add ca certs so we can build the image on vpn
COPY cas.pem /etc/pki/ca-trust/source/anchors/custom_cas.pem
RUN update-ca-trust

RUN dnf install -y epel-release && dnf -y update
RUN dnf install -y curl-minimal git iproute iputils jq make openssl sudo wget which zip zsh \
    gcc gcc-c++ pkgconfig openssl-devel

# GitHub CLI
RUN dnf config-manager --add-repo https://cli.github.com/packages/rpm/gh-cli.repo && \
    dnf install -y gh && \
    gh --version

FROM base as user

# Create the user to mirror the macOS host: UID 501 in group "staff" (GID 20).
# Rocky 10 ships GID 20 as "games" and may already have UID 501 assigned;
# evict both before claiming the slot. Matching the host's real ids means
# bind mounts from /Users/<you> are writable without userns translation —
# which doesn't work through libkrun's virtio-fs anyway.
RUN groupdel games 2>/dev/null || true && \
    if getent passwd ${USER_UID} >/dev/null; then \
        existing_uname=$(getent passwd ${USER_UID} | cut -d: -f1) && \
        userdel -r "$existing_uname" 2>/dev/null || usermod -u 65501 "$existing_uname"; \
    fi && \
    groupadd --gid ${USER_GID} ${USERNAME} && \
    useradd --uid ${USER_UID} --gid ${USER_GID} -m ${USERNAME} -s /bin/bash && \
    echo ${USERNAME} ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/${USERNAME} && \
    chmod 0440 /etc/sudoers.d/${USERNAME}
    
########### NON ROOT FROM HERE DOWN ###########
USER ${USERNAME}

RUN set -eux && \
    sudo mkdir -p /workdir/${REPO_NAME} && \
    sudo chown ${USERNAME}:${USERNAME} /workdir

# envs
ENV HOME "/home/${USERNAME}"
ENV SDK_HOME ${HOME}/sdk
ENV PATH ${HOME}/bin:${PATH}

RUN mkdir -p ${HOME}/bin
    
# rust
ENV PATH ${HOME}/.cargo/bin:${PATH}
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y

# gh
RUN set -eux && \
    export NAME="gh" && \
    export VERSION="2.92.0" && \
    export ARCH="$(arch | sed 's/aarch64/arm64/' | sed 's/x86_64/amd64/')" && \
    cd ${HOME}/bin && \
    curl -k -O -L https://github.com/cli/cli/releases/download/v${VERSION}/${NAME}_${VERSION}_linux_${ARCH}.tar.gz && \
    tar xzf ${NAME}_${VERSION}_linux_${ARCH}.tar.gz && \
    rm ${NAME}_${VERSION}_linux_${ARCH}.tar.gz && \
    ln -s ${NAME}_${VERSION}_linux_${ARCH}/bin/gh gh

# rust
COPY --chown=${USERNAME}:${USERNAME} .bash_pretty ${HOME}/.bash_rust
RUN set -eux && \
    echo "source ${HOME}/.bash_rust" >> ${HOME}/.bashrc

# pretty bash
COPY --chown=${USERNAME}:${USERNAME} .bash_pretty ${HOME}/.bash_pretty
RUN set -eux && \
    echo "source ${HOME}/.bash_pretty" >> ${HOME}/.bashrc
