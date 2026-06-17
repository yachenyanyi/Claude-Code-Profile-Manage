use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, ListState, Paragraph, Table, Tabs, Row, Cell, Wrap},
    Terminal,
};

use crate::config::profile::Profile;
use crate::config::store::{Config, Store};
use crate::shell::generator::Generator;

use super::ui;

/// TUI 模式
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// 普通浏览模式
    Normal,
    /// 添加配置
    Adding,
    /// 编辑配置
    Editing(usize),  // 正在编辑的 profile 索引
    /// 确认删除
    ConfirmDelete(usize),
    /// 帮助
    Help,
}

/// TUI 焦点
#[derive(Debug, Clone, PartialEq)]
pub enum Focus {
    List,
    Detail,
}

/// 应用主状态
pub struct App {
    /// 完整的配置
    pub config: Config,
    /// 当前选中的 profile 索引
    pub selected: usize,
    /// 当前模式
    pub mode: AppMode,
    /// 焦点面板
    pub focus: Focus,
    /// 状态信息
    pub status_message: Option<String>,

    // 内部依赖
    store: Store,
    generator: Generator,
}

impl App {
    /// 创建新 App，加载配置，生成器自动同步
    pub fn new() -> Result<Self> {
        let store = Store::new();
        let generator = Generator::new();
        let config = store.load()?;

        Ok(Self {
            config,
            selected: 0,
            mode: AppMode::Normal,
            focus: Focus::List,
            status_message: None,
            store,
            generator,
        })
    }

    /// 获取当前选中的 profile
    pub fn current_profile(&self) -> Option<&Profile> {
        self.config.profiles.get(self.selected)
    }

    /// 当前选中的 profile 可变引用
    pub fn current_profile_mut(&mut self) -> Option<&mut Profile> {
        self.config.profiles.get_mut(self.selected)
    }

    /// 同步配置到磁盘和包装脚本
    pub fn sync(&mut self) -> Result<()> {
        self.store.save(&self.config)?;
        self.generator.sync(&self.config)?;
        Ok(())
    }

    /// 切换焦点
    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::List => Focus::Detail,
            Focus::Detail => Focus::List,
        };
    }

    /// 切换 profile 启用/禁用状态 => 自动同步
    pub fn toggle_enabled(&mut self) -> Result<()> {
        if let Some(p) = self.current_profile_mut() {
            p.enabled = !p.enabled;
            self.sync()?;
        }
        Ok(())
    }

    /// 删除 profile => 自动同步
    pub fn delete_current(&mut self) -> Result<()> {
        if self.selected < self.config.profiles.len() {
            self.config.profiles.remove(self.selected);
            if self.selected > 0 && self.selected >= self.config.profiles.len() {
                self.selected = self.config.profiles.len().saturating_sub(1);
            }
            self.sync()?;
        }
        Ok(())
    }

    /// 添加 profile => 自动同步
    pub fn add_profile(&mut self, profile: Profile) -> Result<()> {
        // 检查重名
        if self.config.profiles.iter().any(|p| p.name == profile.name) {
            self.status_message = Some(format!("配置「{}」已存在", profile.name));
            return Ok(());
        }
        self.config.profiles.push(profile);
        self.selected = self.config.profiles.len() - 1;
        self.sync()?;
        Ok(())
    }

    /// 更新 profile => 自动同步
    pub fn update_profile(&mut self, index: usize, profile: Profile) -> Result<()> {
        if index < self.config.profiles.len() {
            // 检查重名（排除自身）
            if self.config.profiles.iter().enumerate().any(|(i, p)| p.name == profile.name && i != index) {
                self.status_message = Some(format!("配置「{}」已存在", profile.name));
                return Ok(());
            }
            self.config.profiles[index] = profile;
            self.sync()?;
        }
        Ok(())
    }

    /// 检查 PATH 中是否包含 ~/.local/bin
    pub fn check_path(&self) -> bool {
        if let Ok(path) = std::env::var("PATH") {
            let bin_dir = dirs::home_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("~"))
                .join(".local")
                .join("bin");
            let bin_str = bin_dir.to_string_lossy().to_string();
            path.split(':').any(|p| p == bin_str || p == bin_str.as_str())
        } else {
            false
        }
    }

    /// 处理按键事件
    pub fn handle_key(&mut self, key: KeyCode) -> Result<bool> {
        match key {
            KeyCode::Char('q') => return Ok(false),
            _ => {}
        }
        Ok(true)
    }

    /// 渲染用户界面
    pub fn render(&mut self, f: &mut ratatui::Frame) {
        ui::render(f, self);
    }

    /// 运行 TUI 主循环
    pub fn run(&mut self, terminal: &mut Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>) -> Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !self.handle_key(key.code)? {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
}