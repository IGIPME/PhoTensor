FROM nixos/nix:latest

# 设置环境变量，避免 Nix 在构建时产生过多的 .nix 文件
ENV NIX_CONFIG="experimental-features = nix-command flakes"

# 追加国内 Nix 二进制缓存镜像到 /etc/nix/nix.conf，避免 CNB 构建环境
# 访问默认 cache.nixos.org 慢或超时导致 nix develop 失败
RUN printf '\nsubstituters = https://mirrors.ustc.edu.cn/nix-channels/store https://mirrors.tuna.tsinghua.edu.cn/nix-channels/store https://mirror.sjtu.edu.cn/nix-channels/store https://cache.nixos.org\ntrusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=\n' >> /etc/nix/nix.conf

# 创建工作目录并复制 Nix 所需的清单文件
# 注意：只复制 flake 相关文件，不复制整个仓库；
# CNB 的 docker 构建上下文必须包含仓库根目录的这些文件，
# 若 context 被错误地解析为 .ide/，下面的 COPY 会立即失败并给出明确错误
WORKDIR /workspace
COPY flake.nix flake.lock rust-toolchain.toml .cargo /workspace/

# 使用 Nix 构建开发环境，并将所有依赖安装到系统 PATH 中
# nix develop 会创建一个包含所有 buildInputs 的 shell 环境
# 我们通过 --command 执行命令，将这些工具链接到 /usr/local/bin 以便全局使用
RUN nix develop --command bash -c \
    'mkdir -p /usr/local/bin && for i in $(ls /nix/store/*-rust-*/bin /nix/store/*-python3-*/bin /nix/store/*-nodejs-*/bin /nix/store/*-pnpm-*/bin 2>/dev/null | sort -u); do \
        ln -sf $i /usr/local/bin/; \
    done' \
    && nix develop --command rustc --version \
    && nix develop --command python --version \
    && nix develop --command node --version \
    && nix develop --command pnpm --version

# 设置默认的 shell，使得进入容器时自动激活 Nix 开发环境
# 这里使用 nix develop 来启动一个交互式 shell
CMD ["nix", "develop"]
