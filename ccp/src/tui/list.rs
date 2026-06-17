use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::config::profile::Profile;

/// 渲染左侧配置列表
pub fn render_list(f: &mut Frame, area: Rect, profiles: &[Profile], selected: usize) {
    let items: Vec<ListItem> = profiles
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let enabled_mark = if p.enabled { "●" } else { "○" };
            let color = if p.enabled { Color::Green } else { Color::DarkGray };

            let content = Line::from(vec![
                Span::styled(
                    format!(" {} ", enabled_mark),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    &p.name,
                    Style::default().fg(if p.enabled { Color::White } else { Color::DarkGray }),
                ),
                Span::styled(
                    format!(" ({})", p.vars.get("ANTHROPIC_MODEL").map_or("?", |s| {
                        if s.len() > 20 { &s[..20] } else { s }
                    })),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            if i == selected {
                ListItem::new(content).style(
                    Style::default()
                        .bg(Color::Blue)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(content)
            }
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" 配置列表 ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default().with_selected(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}