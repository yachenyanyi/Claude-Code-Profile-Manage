use anyhow::Result;
use ccpm::tui::app::App;
use ratatui::backend::CrosstermBackend;
use std::io::stdout;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Windows __launch 子命令（内部）：不初始化 color_eyre，不进 TUI
    #[cfg(windows)]
    if args.len() > 1 && args[1] == "__launch" {
        let code = cmd_launch(&args);
        std::process::exit(code);
    }

    color_eyre::install().map_err(|e| anyhow::anyhow!("color_eyre 初始化失败: {e}"))?;

    if args.len() > 1 {
        match args[1].as_str() {
            "list" => cmd_list()?,
            other => {
                eprintln!("未知子命令: {other}");
                eprintln!("用法: ccpm       # 打开 TUI");
                eprintln!("      ccpm list  # 列出配置");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    run_tui()?;
    Ok(())
}

/// Windows: __launch 子命令，由 .cmd wrapper 调用
/// 用法：ccpm.exe __launch <profile_name> -- <claude_args...>
#[cfg(windows)]
fn cmd_launch(args: &[String]) -> i32 {
    // 找 "--" 分隔符
    let sep_idx = args.iter().position(|s| s == "--");
    let (name_part, pass_part) = match sep_idx {
        Some(i) => (&args[2..i], &args[i + 1..]),
        None => (&args[2..], &[] as &[String]),
    };

    let profile_name = match name_part.first() {
        Some(n) => n,
        None => {
            eprintln!("用法：ccpm __launch <profile_name> -- <claude_args>");
            return 1;
        }
    };

    let gen = ccpm::shell::generator::Generator::new();
    gen.launch(profile_name, pass_part)
}

fn run_tui() -> Result<()> {
    // Windows: 自动把自己安装到 .local\bin
    #[cfg(windows)]
    {
        let bin_dir = ccpm::shell::generator::Generator::default_bin_dir();
        let target = bin_dir.join("ccpm.exe");
        if let Ok(exe) = std::env::current_exe() {
            if exe != target {
                let _ = std::fs::create_dir_all(&bin_dir);
                if !target.exists() {
                    let _ = std::fs::copy(&exe, &target);
                }
            }
        }
    }

    // 检查 claude 是否安装
    let claude_installed = std::process::Command::new("claude")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let mut app = App::new()?;

    // PATH 提示 + 一键添加（Windows）
    if !app.check_path() {
        #[cfg(unix)]
        let msg = r#"~/.local/bin 不在 PATH 中，请添加：
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc"#
            .to_string();
        #[cfg(windows)]
        let msg = r#"%USERPROFILE%\.local\bin 不在 PATH 中
按 Y 自动添加到用户环境变量（需重启终端生效），或手动运行：
$env:PATH += ";$env:USERPROFILE\.local\bin""#
            .to_string();
        app.status_message = Some(msg);
        app.needs_path_setup = true; // 新加字段，标记可以按 Y 自动加
    }

    // Claude 未安装提示
    if !claude_installed {
        let existing = app.status_message.take().unwrap_or_default();
        app.status_message = Some(format!(
            "{}\n\n⚠️  未检测到 Claude Code，请先安装：\n    npm install -g @anthropic-ai/claude-code",
            existing
        ));
    }

    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    // Windows 不开启鼠标捕获，否则无法用鼠标选择文字复制
    // Unix 保留，因为 Shift+拖选 在 Linux 终端中能绕过捕获
    #[cfg(unix)]
    crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture)?;
    let mut terminal = ratatui::Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = app.run(&mut terminal);

    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    #[cfg(unix)]
    crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture)?;
    crossterm::terminal::disable_raw_mode()?;

    result?;

    Ok(())
}

fn cmd_list() -> Result<()> {
    let store = ccpm::config::store::Store::new();
    let config = store.load()?;

    if config.profiles.is_empty() {
        println!("暂无配置。运行 ccpm 打开 TUI 添加配置。");
        return Ok(());
    }

    println!("{:20} {:8} {:40} {}", "名称", "状态", "模型", "分组");
    println!("{}", "-".repeat(80));

    for p in &config.profiles {
        let status = if p.enabled { "✓ 启用" } else { "○ 禁用" };
        let model = p.vars.get("ANTHROPIC_MODEL").map_or("-", |s| s);
        let group = p.group.as_deref().unwrap_or("-");
        println!("{:20} {:8} {:40} {}", p.name, status, model, group);
    }

    Ok(())
}