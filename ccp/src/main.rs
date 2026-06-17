use anyhow::Result;
use ccp::tui::app::App;
use ratatui::backend::CrosstermBackend;
use std::io::stdout;

fn main() -> Result<()> {
    let _ = color_eyre::install();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "list" => cmd_list()?,
            other => {
                eprintln!("未知子命令: {other}");
                eprintln!("用法: ccp        # 打开 TUI");
                eprintln!("      ccp list   # 列出配置");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    run_tui()?;
    Ok(())
}

fn run_tui() -> Result<()> {
    let mut app = App::new()?;

    if !app.check_path() {
        app.status_message = Some(
            r#"~/.local/bin 不在 PATH 中，请添加：
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc"#
                .to_string(),
        );
    }

    crossterm::terminal::enable_raw_mode()?;
    let mut terminal = ratatui::Terminal::new(CrosstermBackend::new(stdout()))?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;

    let result = app.run(&mut terminal);

    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    if let Err(e) = result {
        eprintln!("错误: {e}");
    }

    Ok(())
}

fn cmd_list() -> Result<()> {
    let store = ccp::config::store::Store::new();
    let config = store.load()?;

    if config.profiles.is_empty() {
        println!("暂无配置。运行 ccp 打开 TUI 添加配置。");
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