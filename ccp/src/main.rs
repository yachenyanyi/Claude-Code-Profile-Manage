use anyhow::Result;
use ccp::tui::app::App;
use ratatui::{
    backend::CrosstermBackend,
    Terminal,
};
use std::io::stdout;

fn main() -> Result<()> {
    let _ = color_eyre::install();

    let mut app = App::new()?;

    // 如果 PATH 中缺少 ~/.local/bin，启动时检查由 TUI 渲染层提示
    if !app.check_path() {
        app.status_message = Some(
            r#"~/.local/bin 不在 PATH 中，请添加：
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc"#
                .to_string(),
        );
    }

    crossterm::terminal::enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;

    let result = app.run(&mut terminal);

    // 恢复终端
    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    if let Err(e) = result {
        eprintln!("错误: {e}");
    }

    Ok(())
}