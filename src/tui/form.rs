use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
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
    pub var_scroll: usize,  // 环境变量列表滚动偏移
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
            var_scroll: 0,
        }
    }

    /// 预设的环境变量键名（添加模式自动填充）
    const PRESET_VAR_KEYS: &[&'static str] = &[
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL",
        "ANTHROPIC_DEFAULT_HAIKU_MODEL_NAME",
        "ANTHROPIC_DEFAULT_SONNET_MODEL",
        "ANTHROPIC_DEFAULT_SONNET_MODEL_NAME",
        "ANTHROPIC_DEFAULT_OPUS_MODEL",
        "ANTHROPIC_DEFAULT_OPUS_MODEL_NAME",
    ];

    /// 空白表单状态（添加模式），预设常用键名
    pub fn empty() -> Self {
        let vars: Vec<(String, String)> = Self::PRESET_VAR_KEYS
            .iter()
            .map(|k| (k.to_string(), String::new()))
            .collect();
        Self {
            name: String::new(),
            group: String::new(),
            vars,
            field_focus: FormField::Name,
            var_scroll: 0,
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
        p.validate().map_err(|e| e.to_string())?;
        Ok(p)
    }
}

/// 渲染添加/编辑表单
pub fn render_form(f: &mut Frame, area: Rect, app: &mut App) {
    let form_state = match &app.mode {
        AppMode::Adding => &mut app.form_state,
        AppMode::Editing(_) => &mut app.form_state,
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
        .title_bottom(Line::from(Span::styled(
            " 右键粘贴 / Tab切换 ",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::DIM),
        )))
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

    // 计算可见变量数: 每个变量 3 行 (key + value + 空行), 底部加一行 "添加环境变量"
    let vars_inner_h = vars_inner.height as usize;
    let max_visible = if vars_inner_h > 1 {
        (vars_inner_h.saturating_sub(1)) / 3
    } else {
        0
    };

    // 自动滚动: 确保焦点所在的变量可见
    match &form_state.field_focus {
        FormField::VarKey(i) | FormField::VarValue(i) => {
            if *i < form_state.var_scroll {
                form_state.var_scroll = *i;
            } else if *i >= form_state.var_scroll + max_visible && max_visible > 0 {
                form_state.var_scroll = *i - max_visible + 1;
            }
        }
        FormField::AddVar => {
            // AddVar 焦点时滚动到末尾
            let total_vars = form_state.vars.len();
            if total_vars > 0 && form_state.var_scroll + max_visible <= total_vars {
                form_state.var_scroll = total_vars.saturating_sub(max_visible);
            }
        }
        _ => {}
    }

    // 只渲染可见范围内的变量
    let mut var_rows = Vec::new();
    let start = form_state.var_scroll;
    let end = std::cmp::min(start.saturating_add(max_visible), form_state.vars.len());
    for (i, (key, val)) in form_state.vars[start..end].iter().enumerate() {
        let abs_i = start + i;
        let key_focused = form_state.field_focus == FormField::VarKey(abs_i);
        let val_focused = form_state.field_focus == FormField::VarValue(abs_i);

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

        // 变量名行
        let key_display = if key.is_empty() {
            Span::styled("(输入变量名, 例如 ANTHROPIC_MODEL)", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(format!(" {}", key), key_style)
        };
        var_rows.push(Line::from(vec![
            Span::styled(format!(" {}. ╶╴", abs_i + 1), Style::default().fg(Color::DarkGray)),
            Span::styled("KEY", Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            key_display,
        ]));

        // 值行
        let val_display = if val.is_empty() {
            Span::styled("(输入值)", Style::default().fg(Color::DarkGray))
        } else {
            Span::styled(format!(" {}", val), val_style)
        };
        var_rows.push(Line::from(vec![
            Span::raw("     "),
            Span::styled("VAL", Style::default().fg(Color::Cyan)),
            Span::raw("  "),
            val_display,
        ]));

        // 变量间空行
        var_rows.push(Line::from(""));
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