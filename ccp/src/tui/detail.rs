use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
    Frame,
};

use crate::config::profile::Profile;

/// 渲染右侧详情面板
pub fn render_detail(f: &mut Frame, area: Rect, profile: Option<&Profile>) {
    if let Some(p) = profile {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" {} ", p.name))
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(area);

        // 将 block 作为背景渲染，子组件渲染在 inner 内
        f.render_widget(block, area);

        // 分区：基本信息 + 环境变量表 + 状态
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),   // 状态行
                Constraint::Length(2),   // 分组
                Constraint::Min(4),      // 环境变量表
                Constraint::Length(1),   // 底部填充
            ])
            .margin(1)
            .split(inner);

        // 状态行
        let status = if p.enabled {
            Span::styled("● 已启用", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
        } else {
            Span::styled("○ 已禁用", Style::default().fg(Color::DarkGray))
        };
        let status_line = Line::from(vec![
            Span::styled("状态: ", Style::default().fg(Color::Gray)),
            status,
        ]);
        f.render_widget(Paragraph::new(status_line), chunks[0]);

        // 分组
        let group_line = Line::from(vec![
            Span::styled("分组: ", Style::default().fg(Color::Gray)),
            Span::raw(p.group.as_deref().unwrap_or("（未设置）")),
        ]);
        f.render_widget(Paragraph::new(group_line), chunks[1]);

        // 环境变量表
        let mut keys: Vec<&String> = p.vars.keys().collect();
        keys.sort();
        let rows: Vec<Row> = keys
            .iter()
            .map(|k| {
                let val = &p.vars[*k];
                // 隐藏 API Key 部分内容
                let masked = if k.contains("TOKEN") || k.contains("KEY") || k.contains("SECRET") {
                    if val.len() > 8 {
                        format!("{}****{}", &val[..4], &val[val.len()-4..])
                    } else {
                        "****".to_string()
                    }
                } else {
                    val.clone()
                };
                Row::new(vec![
                    Cell::from(Span::styled(
                        k.to_string(),
                        Style::default().fg(Color::Yellow),
                    )),
                    Cell::from(Span::raw(masked)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            vec![
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ],
        )
        .header(
            Row::new(vec![
                Cell::from(Span::styled(
                    "变量名",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Cell::from(Span::styled(
                    "值",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
            ])
            .bottom_margin(1),
        );

        f.render_widget(table, chunks[2]);
    } else {
        // 无 profile 时显示空状态
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" 详情 ")
            .border_style(Style::default().fg(Color::DarkGray));
        let text = Text::from("按 a 添加配置");
        let para = Paragraph::new(text)
            .block(block)
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(para, area);
    }
}