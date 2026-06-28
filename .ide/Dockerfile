# .ide/Dockerfile
# 使用一个包含 Nix 的基础镜像
FROM nixos/nix:latest

# 设置环境变量，避免 Nix 在构建时产生过多的 .nix 文件
ENV NIX_CONFIG="experimental-features = nix-command flakes"

# 创建工作目录
WORKDIR /workspace

# 将项目中的 flake.nix 和 flake.lock 复制到镜像中
# 这样 Nix 就可以根据这些文件来构建环境
COPY flake.nix flake.lock /workspace/

# 使用 Nix 构建开发环境，并将所有依赖安装到系统 PATH 中
# nix develop 会创建一个包含所有 buildInputs 的 shell 环境
# 我们通过 --command 执行命令，将这些工具链接到 /usr/local/bin 以便全局使用
RUN nix develop --command bash -c \
    'for i in $(ls /nix/store/*-rust-*/bin /nix/store/*-python3-*/bin /nix/store/*-nodejs-*/bin /nix/store/*-pnpm-*/bin 2>/dev/null | sort -u); do \
        ln -sf $i /usr/local/bin/; \
    done' \
    && nix develop --command rustc --version \
    && nix develop --command python --version \
    && nix develop --command node --version \
    && nix develop --command pnpm --version

# 设置默认的 shell，使得进入容器时自动激活 Nix 开发环境
# 这里使用 nix develop 来启动一个交互式 shell
CMD ["nix", "develop"]