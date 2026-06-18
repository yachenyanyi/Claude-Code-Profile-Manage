#!/bin/bash
set -euo pipefail

REPO="https://github.com/yachenyanyi/Claude-Code-Profile-Manage.git"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

echo "==> 安装 ccpm — Claude Code Profile Manager"

# 检查依赖
if ! command -v cargo &>/dev/null; then
    echo "错误: 需要 Rust。安装: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# 克隆
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT
echo "==> 克隆仓库..."
git clone --depth 1 "$REPO" "$TMPDIR" 2>/dev/null || {
    echo "错误: 克隆失败，请检查网络或仓库权限"
    exit 1
}

# 构建
echo "==> 编译..."
cd "$TMPDIR/ccp"
cargo build --release 2>&1 | tail -1

# 安装
mkdir -p "$INSTALL_DIR"
cp target/release/ccpm "$INSTALL_DIR/ccpm"
chmod +x "$INSTALL_DIR/ccpm"

echo "==> ✅ 安装完成!"
echo "    二进制: $INSTALL_DIR/ccpm"
echo ""
echo "    运行: ccpm        # 打开 TUI"
echo "          ccpm list   # 列出配置"
echo ""
echo "    确保 PATH 包含 $INSTALL_DIR:"
echo "    echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"