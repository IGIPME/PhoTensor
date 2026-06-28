# PhoTensor 开发环境镜像
# 基于 Ubuntu + Nix（flakes 模式），用于 CNB 构建流水线
#
# 构建：docker build -t photensor-dev .
# 使用：docker run --rm -it photensor-dev
#
# CNB 构建时自动缓存，仅 Dockerfile 变化时重新构建

FROM ubuntu:22.04

# 避免交互式安装提示
ENV DEBIAN_FRONTEND=noninteractive

# ── 系统依赖 ──────────────────────────────────────
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Nix 安装依赖
    curl \
    xz-utils \
    ca-certificates \
    # Git（Nix flakes 需要）
    git \
    # 构建工具链（flake.nix 提供了 clang/lld，但 Nix 安装前需要基础工具）
    build-essential \
    # 清理缓存
    && rm -rf /var/lib/apt/lists/*

# ── 安装 Nix（多用户模式，启用 flakes） ────────────
# 参考：https://nixos.org/download.html
RUN curl -sL https://nixos.org/nix/install | sh -s -- --daemon --yes \
    && echo "experimental-features = nix-command flakes" >> /etc/nix/nix.conf

# 设置 Nix 环境变量（非登录 shell 也能使用）
ENV PATH="/nix/var/nix/profiles/default/bin:${PATH}"

# ── 工作目录 ──────────────────────────────────────
WORKDIR /workspace

# ── 预下载 flake 依赖（利用 Docker 层缓存） ───────
# 将 flake.nix 和 flake.lock 单独复制进来先下载依赖，
# 这样后续代码变更不会触发这层缓存失效。
COPY flake.nix flake.lock rust-toolchain.toml ./
COPY .cargo/config.toml .cargo/config.toml

# 预取 flake inputs 和 devShell 依赖，缓存到 /nix/store
# 注意：nix develop 需要完整的 flake 源码，这里只做预取
RUN nix flake metadata --refresh /workspace \
    && nix develop /workspace --command true 2>/dev/null || true

# 复制项目剩余文件
COPY . .

# ── 默认进入 Nix flake 开发环境 ────────────────────
# CNB 流水线通过 `nix develop --command <cmd>` 执行具体任务
CMD ["nix", "develop", "--command", "bash"]
