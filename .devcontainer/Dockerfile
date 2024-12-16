FROM yaman/rust-builder:latest

RUN apt-get update && apt-get install -y telnet sudo zsh neovim vim redis-tools \
    unzip curl tzdata \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Set timezone to Europe/Istanbul
RUN ln -fs /usr/share/zoneinfo/Europe/Istanbul /etc/localtime && \
    dpkg-reconfigure -f noninteractive tzdata

# Add goose clickhouse migration
RUN curl -fsSL https://raw.githubusercontent.com/pressly/goose/master/install.sh | sh

ENV USER=vscode

# Create a non-root user
RUN useradd -m -s /bin/zsh $USER

# Set up sudo for the new user
RUN echo "$USER ALL=(ALL) NOPASSWD:ALL" > /etc/sudoers.d/$USER && \
    chmod 0440 /etc/sudoers.d/$USER

# Set the user for subsequent commands
USER $USER

# Set the working directory to the user's home
WORKDIR /home/$USER

RUN curl -LO https://github.com/redpanda-data/redpanda/releases/latest/download/rpk-linux-amd64.zip && \
    mkdir -p ~/.local/bin && \
    export PATH="~/.local/bin:$PATH" && \
    unzip rpk-linux-amd64.zip -d ~/.local/bin/

# Add git configuration
RUN git config --global user.email "abdulkadiryaman@gmail.com" && \
    git config --global user.name "yaman"

RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)"

# Set ZSH as the default shell for the user
RUN sudo chsh -s $(which zsh) $USER

RUN mkdir .cargo

RUN touch ~/.cargo/config.toml

# Modify the sccache configuration
RUN echo "[build]" > ~/.cargo/config.toml
RUN echo "rustc-wrapper = '/usr/local/cargo/bin/sccache'" >> ~/.cargo/config.toml
RUN echo "[cache]" >> ~/.cargo/config.toml
RUN echo "dir = '/home/$USER/.cache/sccache'" >> ~/.cargo/config.toml

# Add timezone environment variable
RUN echo "export TZ='Europe/Istanbul'" >> ~/.zshrc

RUN mkdir -p ~/bin

RUN curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to ~/bin

RUN echo "export PATH="$PATH:$HOME/bin"" >> ~/.zshrc

# Add Goose environment variables
RUN echo "export GOOSE_DRIVER=clickhouse" >> ~/.zshrc
RUN echo "export GOOSE_DBSTRING=http://clickhouse:8123" >> ~/.zshrc
RUN echo "export GOOSE_MIGRATION_DIR=/home/vscode/cortex/migrations" >> ~/.zshrc

RUN curl -LO "https://dl.k8s.io/release/$(curl -L -s https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl"
RUN sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl
