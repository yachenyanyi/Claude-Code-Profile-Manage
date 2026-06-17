# ccpm — Claude Code Profile Manager

多提供商 Claude Code 配置管理器。通过 TUI 管理多个 API 配置（环境变量键值对），自动生成 `ccpm-<name>` 包装脚本，每个 profile 有独立的 settings.json 隔离。

## 快速安装

```bash
# 依赖: Rust (https://rustup.rs)
git clone git@github.com:yachenyanyi/Claude-Code-Profile-Manage.git
cd Claude-Code-Profile-Manage/ccp
cargo build --release
cp target/release/ccpm ~/.local/bin/
```

确保 `~/.local/bin` 在 PATH 中：

```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

## 用法

```bash
ccpm            # 打开 TUI 管理配置
ccpm list       # 命令行列出配置
ccpm-<name>     # 启动对应提供商的 Claude Code
```

## 示意图

```
┌─────────────────────────────────────────────────────────┐
│  ccpm — Claude Code Profile Manager           v0.1 帮助 │
├──────────────┬──────────────────────────────────────────┤
│  配置列表     │  详情面板                                │
│              │                                          │
│  ● bailian   │  名称     bailian                        │
│  ○ deepseek  │  分组     -                              │
│              │  状态     ✅ 已启用                       │
│              │  变量:    3 个                            │
│              │  ┌─────────────────────────────────┐      │
│              │  │ ANTHROPIC_AUTH_TOKEN  sk-f****xx │      │
│              │  │ ANTHROPIC_BASE_URL    https://..  │      │
│              │  │ ANTHROPIC_MODEL       qwen3.7..  │      │
│              │  └─────────────────────────────────┘      │
│              │                                          │
│              │  [编辑] [删除] [复制]                     │
├──────────────┴──────────────────────────────────────────┤
│  Tab:切换  ↑↓:选择  e:编辑  a:新增  Space:启用/禁用     │
│  /:搜索  y:复制  ?:帮助  q:退出                        │
└─────────────────────────────────────────────────────────┘
```

## 原理

每个 profile 使用 **HOME 隔离** 让 Claude Code 读取专属 settings.json：

```
shell: export HOME=~/.cache/ccpm/homes/<name>
       symlink ~/.claude/ 内容（共享记忆/项目）
       exec claude "$@"

Claude Code 读取 →  $HOME/.claude/settings.json
               → 隔离 HOME 的 settings.json（profile 专属）
               
记忆/项目数据 → symlink 到真实 HOME，保持共享
```