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

RUN mkdir -p ${HOME}/bin
    
# rust
ENV PATH ${HOME}/.cargo/bin:${PATH}
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | bash -s -- -y

# gh
RUN set -eux && \
    export NAME="gh" && \
    export VERSION="2.67.0" && \
    export ARCH="$(arch | sed 's/aarch64/arm64/' | sed 's/x86_64/amd64/')" && \
    cd ${HOME}/bin && \
    curl -k -O -L https://github.com/cli/cli/releases/download/v${VERSION}/${NAME}_${VERSION}_linux_${ARCH}.tar.gz && \
    tar xzf ${NAME}_${VERSION}_linux_${ARCH}.tar.gz && \
    rm ${NAME}_${VERSION}_linux_${ARCH}.tar.gz && \
    ln -s ${NAME}_${VERSION}_linux_${ARCH}/bin/gh gh

# pretty bash
COPY --chown=${USERNAME}:${USERNAME} .bash_pretty ${HOME}/.bash_rust
RUN set -eux && \
    echo "source ${HOME}/.bash_rust" >> ${HOME}/.bashrc

# pretty bash
COPY --chown=${USERNAME}:${USERNAME} .bash_pretty ${HOME}/.bash_pretty
RUN set -eux && \
    echo "source ${HOME}/.bash_pretty" >> ${HOME}/.bashrc

