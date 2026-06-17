use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};


use super::app::{App, AppMode};
use super::detail::render_detail;
use super::list::render_list;
use super::form::render_form;

/// 主渲染函数——将整个 App 状态渲染到屏幕上
pub fn render(f: &mut Frame, app: &mut App) {
    let area = f.area();

    // 标题栏（固定高度 1）
    let title_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    // 顶部标题
    let title = if app.status_message.is_some() {
        Line::from(vec![
            Span::raw(" ccp — Claude Code Profiles  "),
            Span::styled(
                app.status_message.as_ref().unwrap(),
                Style::default().fg(Color::Yellow),
            ),
        ])
    } else {
        Line::from(Span::raw(" ccp — Claude Code Profiles"))
    };
    f.render_widget(Paragraph::new(title).style(Style::default().fg(Color::Cyan)), title_area[0]);

    // 主内容区域（面板 + 底部操作栏）
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(title_area[1]);

    // 底部操作栏
    let help_text = match app.mode {
        AppMode::Normal => {
            " Tab:切换焦点  ↑↓:选择  e:编辑  a:新增  Space:启用/禁用  d:删除  /:搜索  ?:帮助  q:退出 "
        }
        AppMode::Adding | AppMode::Editing(_) => {
            " Tab:切换字段  Enter:保存  Esc:取消 "
        }
        AppMode::ConfirmDelete(_) => {
            " y:确认删除  n:取消 "
        }
        AppMode::Help => {
            " Esc 或 ?:关闭帮助 "
        }
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            help_text,
            Style::default().fg(Color::Black).bg(Color::White),
        ))),
        main_chunks[1],
    );

    // 面板区域
    match &app.mode {
        AppMode::Adding | AppMode::Editing(_) => {
            // 表单模式：全屏表单
            render_form(f, main_chunks[0], app);
        }
        AppMode::Help => {
            // 帮助弹窗
            render_help(f, main_chunks[0]);
        }
        AppMode::ConfirmDelete(_index) => {
            // 确认删除弹窗
            render_confirm_delete(f, main_chunks[0], app);
        }
        AppMode::Normal => {
            // 正常双面板布局
            let panels = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(main_chunks[0]);

            render_list(f, panels[0], &app.config.profiles, app.selected);
            render_detail(f, panels[1], app.current_profile());
        }
    }
}

/// 帮助弹窗
fn render_help(f: &mut Frame, area: Rect) {
    let help_text = Text::from(vec![
        Line::from(Span::styled("快捷键帮助", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(vec![Span::styled("a", Style::default().fg(Color::Yellow)), Span::raw(" — 添加配置")]),
        Line::from(vec![Span::styled("e", Style::default().fg(Color::Yellow)), Span::raw(" — 编辑选中配置")]),
        Line::from(vec![Span::styled("d", Style::default().fg(Color::Yellow)), Span::raw(" — 删除选中配置")]),
        Line::from(vec![Span::styled("Space", Style::default().fg(Color::Yellow)), Span::raw(" — 启用/禁用")]),
        Line::from(vec![Span::styled("Tab", Style::default().fg(Color::Yellow)), Span::raw(" — 切换焦点面板")]),
        Line::from(vec![Span::styled("↑/↓", Style::default().fg(Color::Yellow)), Span::raw(" — 上/下选择")]),
        Line::from(vec![Span::styled("/", Style::default().fg(Color::Yellow)), Span::raw(" — 搜索")]),
        Line::from(vec![Span::styled("q / Esc", Style::default().fg(Color::Yellow)), Span::raw(" — 退出/取消")]),
        Line::from(""),
        Line::from(Span::styled("配置说明:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from("在 TUI 中编辑配置后，系统自动同步到 ~/.local/bin/ccp-<name>"),
        Line::from("之后在终端中直接使用 ccp-<name> 命令启动 Claude Code"),
    ]);

    let block = Clear;
    f.render_widget(block, area);

    let para = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 帮助 ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().bg(Color::Black))
        .alignment(ratatui::layout::Alignment::Left);
    f.render_widget(para, area);
}

/// 确认删除弹窗
fn render_confirm_delete(f: &mut Frame, area: Rect, app: &App) {
    let text = match app.current_profile() {
        Some(p) => format!("确认删除配置「{}」？", p.name),
        None => "确认删除此配置？".to_string(),
    };

    let para = Paragraph::new(Text::from(vec![
        Line::from(Span::styled(text, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("y 确认  |  n 取消", Style::default().fg(Color::Gray))),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" 确认删除 ")
            .border_style(Style::default().fg(Color::Red)),
    )
    .style(Style::default().bg(Color::Black))
    .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(Clear, area);
    f.render_widget(para, area);
}