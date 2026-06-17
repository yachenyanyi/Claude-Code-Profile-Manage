use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::config::profile::Profile;
use super::app::{App, AppMode};

/// 表单字段焦点
#[derive(Debug, Clone, PartialEq)]
pub enum FormField {
    Name,
    Group,
    VarKey(usize),   // 环境变量 key 输入框索引
    VarValue(usize), // 环境变量 value 输入框索引
    AddVar,          // 添加变量的按钮
}

/// 表单状态
#[derive(Debug, Clone)]
pub struct FormState {
    pub name: String,
    pub group: String,
    pub vars: Vec<(String, String)>,  // (key, value)
    pub field_focus: FormField,
}

impl FormState {
    /// 从 profile 创建表单状态（编辑模式）
    pub fn from_profile(p: &Profile) -> Self {
        let mut vars: Vec<(String, String)> = p.vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            name: p.name.clone(),
            group: p.group.clone().unwrap_or_default(),
            vars,
            field_focus: FormField::Name,
        }
    }

    /// 空白表单状态（添加模式）
    pub fn empty() -> Self {
        Self {
            name: String::new(),
            group: String::new(),
            vars: vec![(String::new(), String::new())],  // 默认一行空变量
            field_focus: FormField::Name,
        }
    }

    /// 将表单状态转换为 Profile
    pub fn to_profile(&self) -> Result<Profile, String> {
        if self.name.trim().is_empty() {
            return Err("名称不能为空".to_string());
        }
        let mut vars = std::collections::HashMap::new();
        for (k, v) in &self.vars {
            let k = k.trim();
            let v = v.trim();
            if !k.is_empty() && !v.is_empty() {
                vars.insert(k.to_string(), v.to_string());
            }
        }
        let mut p = Profile::new(self.name.trim().to_string());
        p.group = if self.group.trim().is_empty() {
            None
        } else {
            Some(self.group.trim().to_string())
        };
        p.vars = vars;
        Ok(p)
    }
}

/// 渲染添加/编辑表单
pub fn render_form(f: &mut Frame, area: Rect, app: &App) {
    let form_state = match &app.mode {
        AppMode::Adding => &app.form_state,
        AppMode::Editing(_) => &app.form_state,
        _ => return,
    };

    let title = match &app.mode {
        AppMode::Adding => " 添加配置 ",
        AppMode::Editing(_) => " 编辑配置 ",
        _ => " 配置表单 ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // name
            Constraint::Length(3),  // group
            Constraint::Min(2),     // vars
        ])
        .margin(1)
        .split(inner);

    // Name 字段
    let name_style = if form_state.field_focus == FormField::Name {
        Style::default().bg(Color::Blue).fg(Color::White)
    } else {
        Style::default()
    };
    let name_content = if form_state.name.is_empty() {
        Paragraph::new("")
            .block(Block::default().borders(Borders::ALL).title(" 名称 * "))
    } else {
        Paragraph::new(Line::from(Span::raw(&form_state.name)))
            .block(Block::default().borders(Borders::ALL).title(" 名称 * "))
    };
    f.render_widget(name_content.style(name_style), chunks[0]);

    // Group 字段
    let group_style = if form_state.field_focus == FormField::Group {
        Style::default().bg(Color::Blue).fg(Color::White)
    } else {
        Style::default()
    };
    let group_content = if form_state.group.is_empty() {
        Paragraph::new("").block(Block::default().borders(Borders::ALL).title(" 分组 "))
    } else {
        Paragraph::new(Line::from(Span::raw(&form_state.group)))
            .block(Block::default().borders(Borders::ALL).title(" 分组 "))
    };
    f.render_widget(group_content.style(group_style), chunks[1]);

    // 环境变量列表
    let vars_block = Block::default()
        .borders(Borders::ALL)
        .title(" 环境变量 (Tab 添加/删除) ");
    let vars_inner = vars_block.inner(chunks[2]);
    f.render_widget(vars_block, chunks[2]);

    // 每个变量占一行
    let mut var_rows = Vec::new();
    for (i, (key, val)) in form_state.vars.iter().enumerate() {
        let key_focused = form_state.field_focus == FormField::VarKey(i);
        let val_focused = form_state.field_focus == FormField::VarValue(i);

        let key_display = if key.is_empty() {
            Span::raw("(输入变量名)")
        } else {
            Span::raw(key)
        };
        let val_display = if val.is_empty() {
            Span::raw("(输入值)")
        } else {
            Span::raw(val)
        };

        let key_style = if key_focused {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default().fg(Color::Yellow)
        };
        let val_style = if val_focused {
            Style::default().bg(Color::Blue).fg(Color::White)
        } else {
            Style::default()
        };

        var_rows.push(Line::from(vec![
            Span::styled(format!("  {}. ", i + 1), Style::default().fg(Color::DarkGray)),
            Span::styled("KEY: ", Style::default().fg(Color::Gray)),
            Span::styled(key_display.to_string(), key_style),
            Span::raw("  "),
            Span::styled("VAL: ", Style::default().fg(Color::Gray)),
            Span::styled(val_display.to_string(), val_style),
        ]));
    }

    // 添加新变量行
    let add_style = if form_state.field_focus == FormField::AddVar {
        Style::default().bg(Color::Blue).fg(Color::White)
    } else {
        Style::default().fg(Color::Green)
    };
    var_rows.push(Line::from(Span::styled(
        "  + 添加环境变量",
        add_style,
    )));

    let vars_para = Paragraph::new(Text::from(var_rows));
    f.render_widget(vars_para, vars_inner);
}