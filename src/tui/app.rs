use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEvent, MouseEventKind};
use ratatui::Terminal;

use crate::config::profile::Profile;
use crate::config::store::{Config, Store};
use crate::shell::generator::Generator;
use std::process::Command;

use super::form::{FormField, FormState};
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
    /// 表单状态（添加/编辑时使用）
    pub form_state: FormState,
    /// 搜索关键字
    pub search_query: String,
    /// 是否在搜索模式
    pub searching: bool,
    /// Windows: 是否可以按 Y 一键添加 PATH
    pub needs_path_setup: bool,

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
            form_state: FormState::empty(),
            search_query: String::new(),
            searching: false,
            needs_path_setup: false,
            store,
            generator,
        })
    }

    /// 使用指定的 Store 和 Generator 创建（测试环境隔离）
    pub fn new_with(store: Store, generator: Generator) -> Result<Self> {
        let config = store.load()?;
        Ok(Self {
            config,
            selected: 0,
            mode: AppMode::Normal,
            focus: Focus::List,
            status_message: None,
            form_state: FormState::empty(),
            search_query: String::new(),
            searching: false,
            needs_path_setup: false,
            store,
            generator,
        })
    }

    /// 获取过滤后的配置
    pub fn filtered_profiles(&self) -> Vec<&Profile> {
        if self.search_query.is_empty() {
            self.config.profiles.iter().collect()
        } else {
            let q = self.search_query.to_lowercase();
            self.config
                .profiles
                .iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&q)
                        || p.group.as_deref().unwrap_or("").to_lowercase().contains(&q)
                        || p.vars.iter().any(|(k, v)| {
                            k.to_lowercase().contains(&q) || v.to_lowercase().contains(&q)
                        })
                })
                .collect()
        }
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
            anyhow::bail!("配置「{}」已存在", profile.name);
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
                anyhow::bail!("配置「{}」已存在", profile.name);
            }
            self.config.profiles[index] = profile;
            self.sync()?;
        }
        Ok(())
    }

    /// 检查 PATH 中是否包含包装脚本所在目录（跨平台）
    pub fn check_path(&self) -> bool {
        let Ok(path) = std::env::var("PATH") else {
            return false;
        };
        let bin_dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("~"))
            .join(".local")
            .join("bin");
        // env::split_paths 在 Unix 按 ':'，在 Windows 按 ';'（且能处理 "..." 段）。
        // PathBuf 比较自动处理分隔符差异；Windows 路径不区分大小写需额外处理。
        std::env::split_paths(&path).any(|p| {
            if p == bin_dir {
                return true;
            }
            #[cfg(windows)]
            {
                p.to_string_lossy().eq_ignore_ascii_case(&*bin_dir.to_string_lossy())
            }
            #[cfg(not(windows))]
            {
                false
            }
        })
    }

    /// 处理按键事件，返回 false 则退出循环
    pub fn handle_key(&mut self, key: KeyCode) -> Result<bool> {
        match &self.mode.clone() {
            AppMode::Normal => self.handle_normal_key(key),
            AppMode::Adding | AppMode::Editing(_) => self.handle_form_key(key),
            AppMode::ConfirmDelete(_) => self.handle_confirm_key(key),
            AppMode::Help => {
                self.mode = AppMode::Normal;
                Ok(true)
            }
        }
    }

    // ----- Normal 模式按键处理 -----
    fn handle_normal_key(&mut self, key: KeyCode) -> Result<bool> {
        // 搜索模式下的字符输入处理
        if self.searching {
            match key {
                KeyCode::Char(c) => self.search_query.push(c),
                KeyCode::Backspace => { self.search_query.pop(); }
                KeyCode::Esc => {
                    self.searching = false;
                    self.search_query.clear();
                }
                KeyCode::Enter => { self.searching = false; }
                _ => {}
            }
            return Ok(true);
        }

        match key {
            #[cfg(windows)]
            KeyCode::Char('y') | KeyCode::Char('Y') if self.needs_path_setup => {
                // Windows 一键添加 PATH 到用户环境变量
                use std::env;
                if let Ok(mut path) = env::var("PATH") {
                    let bin_dir = Generator::default_bin_dir();
                    let bin_str = bin_dir.to_string_lossy();
                    if !path.contains(&*bin_str) {
                        path.push_str(&format!(";{}", bin_str));
                        // 写入用户环境变量
                        env::set_var("PATH", &path);
                        // 持久化到注册表
                        let _ = std::process::Command::new("setx")
                            .args(["PATH", &path])
                            .status();
                    }
                }
                self.status_message = Some("✅ PATH 已添加！请重启终端生效".to_string());
                self.needs_path_setup = false;
            }
            KeyCode::Char('q') => return Ok(false),
            KeyCode::Char('?') => self.mode = AppMode::Help,
            KeyCode::Char('/') => {
                self.searching = true;
                self.search_query.clear();
            }
            KeyCode::Tab => self.toggle_focus(),
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focus == Focus::List && self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.focus == Focus::List
                    && self.selected + 1 < self.config.profiles.len()
                {
                    self.selected += 1;
                }
            }
            KeyCode::Char(' ') => {
                self.remap_selected_if_filtered();
                self.toggle_enabled()?;
            }
            KeyCode::Char('d') => {
                self.remap_selected_if_filtered();
                if !self.config.profiles.is_empty() {
                    self.mode = AppMode::ConfirmDelete(self.selected);
                }
            }
            KeyCode::Char('y') => {
                self.remap_selected_if_filtered();
                if let Some(p) = self.current_profile() {
                    let mut lines: Vec<String> = p.vars.iter()
                        .map(|(k, v)| format!("export {}='{}'", k, v.replace('\'', "'\\''")))
                        .collect();
                    lines.sort();
                    let text = lines.join("\n");
                    let copied = copy_to_clipboard(&text);
                    if copied {
                        self.status_message = Some(format!("配置「{}」的环境变量已复制到剪贴板", p.name));
                    } else {
                        self.status_message = Some("复制失败: 请安装 xclip 或 wl-copy".to_string());
                    }
                }
            }
            KeyCode::Char('a') => {
                self.form_state = FormState::empty();
                self.mode = AppMode::Adding;
            }
            KeyCode::Char('e') => {
                self.remap_selected_if_filtered();
                if let Some(p) = self.current_profile() {
                    self.form_state = FormState::from_profile(p);
                    self.mode = AppMode::Editing(self.selected);
                }
            }
            _ => {}
        }
        Ok(true)
    }

    /// 当搜索激活时，如果当前选中的 profile 不在过滤结果中，
    /// 将选中索引重映射到第一个可见的 profile。
    fn remap_selected_if_filtered(&mut self) {
        if self.searching && !self.search_query.is_empty() {
            let filtered_names: Vec<String> = self
                .filtered_profiles()
                .iter()
                .map(|p| p.name.clone())
                .collect();
            let visible = self
                .config
                .profiles
                .get(self.selected)
                .map(|p| filtered_names.contains(&p.name))
                .unwrap_or(false);
            if !visible {
                if let Some(first) = filtered_names.first() {
                    if let Some(idx) = self
                        .config
                        .profiles
                        .iter()
                        .position(|p| &p.name == first)
                    {
                        self.selected = idx;
                    }
                }
            }
        }
    }

    // ----- 表单模式按键处理 -----
    fn handle_form_key(&mut self, key: KeyCode) -> Result<bool> {
        let form = &mut self.form_state;
        match key {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.status_message = None;
            }
            KeyCode::Enter => {
                // 尝试保存
                match form.to_profile() {
                    Ok(profile) => {
                        let result = match &self.mode.clone() {
                            AppMode::Adding => self.add_profile(profile),
                            AppMode::Editing(idx) => self.update_profile(*idx, profile),
                            _ => Ok(()),
                        };
                        if result.is_ok() {
                            self.status_message = None;
                            self.mode = AppMode::Normal;
                        } else if let Err(e) = result {
                            self.status_message = Some(format!("保存失败: {}", e));
                        }
                    }
                    Err(e) => {
                        self.status_message = Some(e);
                    }
                }
            }
            KeyCode::Tab => {
                // 循环切换字段焦点
                form.field_focus = match &form.field_focus {
                    FormField::Name => FormField::Group,
                    FormField::Group => {
                        if form.vars.is_empty() {
                            FormField::AddVar
                        } else {
                            FormField::VarKey(0)
                        }
                    }
                    FormField::VarKey(i) => FormField::VarValue(*i),
                    FormField::VarValue(i) => {
                        if *i + 1 < form.vars.len() {
                            FormField::VarKey(*i + 1)
                        } else {
                            FormField::AddVar
                        }
                    }
                    FormField::AddVar => FormField::Name,
                };
            }
            KeyCode::Backspace => {
                // 删除当前字段的最后一个字符
                match &form.field_focus.clone() {
                    FormField::Name => { form.name.pop(); }
                    FormField::Group => { form.group.pop(); }
                    FormField::VarKey(i) => { form.vars[*i].0.pop(); }
                    FormField::VarValue(i) => { form.vars[*i].1.pop(); }
                    FormField::AddVar => {}
                }
            }
            KeyCode::Char(c) => {
                match &form.field_focus.clone() {
                    FormField::Name => form.name.push(c),
                    FormField::Group => form.group.push(c),
                    FormField::VarKey(i) => form.vars[*i].0.push(c),
                    FormField::VarValue(i) => form.vars[*i].1.push(c),
                    FormField::AddVar => {
                        if c == ' ' || c == '\t' {
                            form.vars.push((String::new(), String::new()));
                            form.field_focus = FormField::VarKey(form.vars.len() - 1);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(true)
    }

    // ----- 确认删除模式按键处理 -----
    fn handle_confirm_key(&mut self, key: KeyCode) -> Result<bool> {
        match key {
            KeyCode::Char('y') | KeyCode::Enter => {
                self.delete_current()?;
                self.mode = AppMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.mode = AppMode::Normal;
            }
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

            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        if !self.handle_key(key.code)? {
                            break;
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    self.handle_mouse(mouse);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// 处理鼠标事件
    fn handle_mouse(&mut self, mouse: MouseEvent) {
        // 表单模式下滚轮滚动环境变量列表
        if matches!(self.mode, AppMode::Adding | AppMode::Editing(_)) {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    if self.form_state.var_scroll > 0 {
                        self.form_state.var_scroll -= 1;
                    }
                }
                MouseEventKind::ScrollDown => {
                    let total = self.form_state.vars.len();
                    let max_visible = 5; // 粗略估计，渲染时会精确调整
                    if self.form_state.var_scroll + max_visible < total {
                        self.form_state.var_scroll += 1;
                    }
                }
                _ => {}
            }
        }
    }
}

/// 将文本复制到系统剪贴板
fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(windows)]
    {
        // Windows: 用 arboard（支持 CJK 等 Unicode）
        if let Ok(mut cb) = arboard::Clipboard::new() {
            return cb.set_text(text).is_ok();
        }
        return false;
    }

    #[cfg(unix)]
    {
        // 尝试 xclip (X11)
        if let Ok(mut child) = Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
                let _ = stdin.flush();
            }
            return child.wait().is_ok();
        }

        // 尝试 wl-copy (Wayland)
        if let Ok(mut child) = Command::new("wl-copy")
            .stdin(std::process::Stdio::piped())
            .spawn()
        {
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                let _ = stdin.write_all(text.as_bytes());
                let _ = stdin.flush();
            }
            return child.wait().is_ok();
        }

        false
    }
}