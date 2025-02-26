FROM rockylinux:8.5 as base

ARG REPO_NAME=rust-play
ARG USERNAME=rusty
ARG USER_UID=1000
ARG USER_GID=$USER_UID

RUN set -eux -o pipefail && \
    dnf install -y curl dnf-utils dnf-plugins-core epel-release git jq ncurses sudo which rsync && \
    dnf config-manager --set-enabled powertools && \ 
    dnf install -y ca-certificates cmake gcc gcc-c++ glib2-devel xz

# envs
ENV INSTALL_PATH "/usr/local"
ENV PATH ${HOME}/bin:${PATH}

# Create the user
RUN groupadd --gid ${USER_GID} ${USERNAME} && \
  useradd --uid ${USER_UID} --gid ${USER_GID} -m ${USERNAME} -s /bin/zsh && \
  echo ${USERNAME} ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/${USERNAME} && \
  chmod 0440 /etc/sudoers.d/${USERNAME}

########### NON ROOT FROM HERE DOWN ###########
FROM base as user

USER ${USERNAME}

RUN set -eux && \
    sudo mkdir -p /workdir/${REPO_NAME} && \
    sudo chown ${USERNAME}:${USERNAME} /workdir

# envs
ENV HOME "/home/${USERNAME}"
ENV SDK_HOME ${HOME}/sdk
ENV PATH ${HOME}/bin:${PATH}

# zig
ENV VERSION "0.13.0"
ENV PATH ${GOPATH}/bin:${HOME}/sdk/zig:${PATH}
RUN set -eux && \
    export ARCH="$(arch | sed 's/aarch64/aarch64/' | sed 's/x86_64/x86_64/')" && \
    cd /tmp && mkdir -p ${SDK_HOME} && \
    curl -o zig.tar.xz -L https://ziglang.org/download/${VERSION}/zig-linux-${ARCH}-${VERSION}.tar.xz && \
    tar xf zig.tar.xz && mv zig-linux-${ARCH}-${VERSION} ${SDK_HOME} && rm /tmp/zig.tar.xz && \
    cd ${SDK_HOME} && ln -s zig-linux-${ARCH}-${VERSION} zig
   
# pretty bash
COPY --chown=${USERNAME}:${USERNAME} .bash_pretty ${HOME}/.bash_pretty
RUN set -eux && \
    echo "source ${HOME}/.bash_pretty" >> ${HOME}/.bashrc

