use anyhow::{Context, Result};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::config::profile::Profile;
use crate::config::store::Config;

/// Shell 包装脚本生成器
#[derive(Debug, Clone)]
pub struct Generator {
    bin_dir: PathBuf,
    homes_dir: PathBuf,
    /// 真实 HOME 路径（测试时可覆写）
    real_home: PathBuf,
}

impl Generator {
    /// 返回默认 bin 目录（供 app.rs check_path 复用）
    pub fn default_bin_dir() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        home.join(".local").join("bin")
    }

    /// 使用默认路径
    pub fn new() -> Self {
        let real_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        let bin_dir = Self::default_bin_dir();
        #[cfg(unix)]
        let homes_dir = real_home.join(".cache").join("ccpm").join("homes");
        #[cfg(windows)]
        let homes_dir = dirs::cache_dir()
            .unwrap_or_else(|| real_home.clone())
            .join("ccpm")
            .join("homes");
        Self { bin_dir, homes_dir, real_home }
    }

    /// 覆写真实 HOME 路径（用于测试 link_isolated_home）
    pub fn with_real_home(mut self, real_home: PathBuf) -> Self {
        self.real_home = real_home;
        self
    }

    /// 使用指定路径（用于测试）
    pub fn with_dir(bin_dir: PathBuf) -> Self {
        let homes_dir = bin_dir.parent().unwrap_or(&bin_dir).join("homes");
        let real_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        Self { bin_dir, homes_dir, real_home }
    }

    /// 使用指定路径（完整隔离，用于测试）
    pub fn with_dirs(bin_dir: PathBuf, homes_dir: PathBuf) -> Self {
        let real_home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        Self { bin_dir, homes_dir, real_home }
    }

    /// 返回某个 profile 对应的脚本路径
    pub fn path_for(&self, name: &str) -> PathBuf {
        let mut filename = format!("ccpm-{}", name);
        #[cfg(windows)]
        filename.push_str(".cmd");
        self.bin_dir.join(filename)
    }

    /// 返回 profile 隔离 HOME 目录
    fn home_dir_for(&self, name: &str) -> PathBuf {
        self.homes_dir.join(name)
    }

    /// 返回 profile 隔离 HOME 下的 settings.json 路径
    fn settings_path_for(&self, name: &str) -> PathBuf {
        self.home_dir_for(name).join(".claude").join("settings.json")
    }

    /// 读取默认 Claude Code settings.json
    fn read_default_settings() -> Option<serde_json::Value> {
        let home = dirs::home_dir()?;
        for path in &[
            home.join(".claude").join("claude_code_settings.json"),
            home.join(".claude").join("settings.json"),
        ] {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(val) = serde_json::from_str(&content) {
                    return Some(val);
                }
            }
        }
        None
    }

    /// 生成 profile 专属 settings.json 内容 = 默认 settings + profile env vars
    fn generate_profile_settings(&self, profile: &Profile) -> serde_json::Value {
        let mut settings = Self::read_default_settings()
            .unwrap_or(serde_json::json!({}));

        // 合并 env 段：profile 的 vars 覆盖默认值
        match settings.get_mut("env") {
            Some(env_obj) if env_obj.is_object() => {
                let env_map = env_obj.as_object_mut().unwrap();
                for (k, v) in &profile.vars {
                    env_map.insert(k.clone(), serde_json::json!(v));
                }
            }
            _ => {
                let env_map: serde_json::Map<String, serde_json::Value> = profile.vars
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::json!(v)))
                    .collect();
                settings["env"] = serde_json::json!(env_map);
            }
        }

        settings
    }

    /// 为 profile 创建隔离 HOME 目录并写入 settings.json
    fn install_isolated_home(&self, profile: &Profile) -> Result<()> {
        let home_dir = self.home_dir_for(&profile.name);
        let claude_dir = home_dir.join(".claude");

        std::fs::create_dir_all(&claude_dir)
            .with_context(|| format!("创建隔离 HOME 失败: {}", claude_dir.display()))?;

        let settings_path = self.settings_path_for(&profile.name);
        let settings = self.generate_profile_settings(profile);
        let content = serde_json::to_string_pretty(&settings)
            .with_context(|| "序列化 settings.json 失败")?;

        std::fs::write(&settings_path, &content)
            .with_context(|| format!("写入 settings.json 失败: {}", settings_path.display()))?;

        Ok(())
    }

    /// 将一个 profile 安装为包装脚本
    pub fn install(&self, profile: &Profile) -> Result<()> {
        std::fs::create_dir_all(&self.bin_dir)
            .with_context(|| format!("创建目录失败: {}", self.bin_dir.display()))?;

        // 创建隔离 HOME
        self.install_isolated_home(profile)?;

        let script_content = self.generate_script(profile);
        let script_path = self.path_for(&profile.name);

        std::fs::write(&script_path, &script_content)
            .with_context(|| format!("写入脚本失败: {}", script_path.display()))?;

        // chmod +x（仅 Unix；Windows 不需要执行位，扩展名 .cmd 决定可执行性）
        #[cfg(unix)]
        {
            let mut perms = std::fs::metadata(&script_path)
                .with_context(|| format!("读取脚本元数据失败: {}", script_path.display()))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms)
                .with_context(|| format!("设置脚本权限失败: {}", script_path.display()))?;
        }

        // Windows：把 ccpm.exe 自己复制到 bin_dir（不存在时或大小变化时），供 .cmd 调用
        #[cfg(windows)]
        self.install_self()?;

        Ok(())
    }

    /// Windows: 把当前 ccpm.exe 复制到 bin_dir
    #[cfg(windows)]
    fn install_self(&self) -> Result<()> {
        let exe_path = std::env::current_exe()
            .with_context(|| "无法获取当前 ccpm.exe 路径")?;
        let target = self.bin_dir.join("ccpm.exe");
        if !target.exists() {
            std::fs::copy(&exe_path, &target)
                .with_context(|| format!("复制 ccpm.exe 失败: {}", target.display()))?;
            return Ok(());
        }
        // 大小不同才重写，避免正在运行时锁文件问题（用户退出后重启 TUI 会更新）
        if let Ok(s1) = std::fs::metadata(&exe_path) {
            if let Ok(s2) = std::fs::metadata(&target) {
                if s1.len() == s2.len() {
                    return Ok(());
                }
            }
        }
        let _ = std::fs::remove_file(&target);
        std::fs::copy(&exe_path, &target)
            .with_context(|| format!("覆盖 ccpm.exe 失败（可能正在运行中）: {}", target.display()))?;
        Ok(())
    }

    /// Windows: 每次启动时刷新真实 HOME → 隔离 HOME 的链接（目录→junction，文件→hard link）
    /// 跳过 settings.json（由 install_isolated_home 生成 profile 专属版本）
    #[cfg(windows)]
    pub fn link_isolated_home(&self, profile_name: &str) -> Result<()> {
        let iso_home = self.home_dir_for(profile_name);
        let iso_claude = iso_home.join(".claude");
        let real_claude = self.real_home.join(".claude");
        let real_claude_json = self.real_home.join(".claude.json");
        let iso_claude_json = iso_home.join(".claude.json");

        // 真实 .claude 不存在（首次用户）→ 不报错，只确保隔离目录存在
        if !real_claude.exists() {
            std::fs::create_dir_all(&iso_claude)
                .with_context(|| format!("创建隔离 .claude 失败: {}", iso_claude.display()))?;
        } else {
            std::fs::create_dir_all(&iso_claude)
                .with_context(|| format!("创建隔离 .claude 失败: {}", iso_claude.display()))?;

            // 遍历 real/.claude/*
            for entry in std::fs::read_dir(&real_claude)
                .with_context(|| format!("读取 .claude 失败: {}", real_claude.display()))?
            {
                let entry = entry?;
                let ft = entry.file_type()?;
                let name = entry.file_name();
                if name == "settings.json" {
                    continue; // 跳过，专属版本已由 install_isolated_home 生成
                }
                let iso_target = iso_claude.join(&name);
                // 删除旧链接（可能是 junction、hard link 或普通文件）
                if iso_target.exists() {
                    if ft.is_dir() {
                        // junction 删除用 remove_dir
                        let _ = std::fs::remove_dir(&iso_target);
                        // fallback: 是复制的目录 → remove_dir_all
                        let _ = std::fs::remove_dir_all(&iso_target);
                    } else {
                        let _ = std::fs::remove_file(&iso_target);
                    }
                }

                if ft.is_dir() {
                    // 目录 → junction
                    junction::create(entry.path(), &iso_target)
                        .with_context(|| format!("创建 junction 失败: {} → {}", entry.path().display(), iso_target.display()))?;
                } else if ft.is_file() {
                    // 文件 → hard link，跨卷 fallback 到 copy
                    if std::fs::hard_link(entry.path(), &iso_target).is_err() {
                        std::fs::copy(entry.path(), &iso_target)
                            .with_context(|| format!("复制文件失败（跨卷 fallback）: {} → {}", entry.path().display(), iso_target.display()))?;
                    }
                }
            }
        }

        // .claude.json: hard link → copy fallback
        if real_claude_json.exists() {
            let _ = std::fs::remove_file(&iso_claude_json);
            if std::fs::hard_link(&real_claude_json, &iso_claude_json).is_err() {
                let _ = std::fs::copy(&real_claude_json, &iso_claude_json);
            }
        }

        Ok(())
    }

    /// Windows: 启动 claude（__launch 入口），返回退出码
    #[cfg(windows)]
    pub fn launch(&self, profile_name: &str, passthrough_args: &[String]) -> i32 {
        let store = crate::config::store::Store::new();
        let config = match store.load() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("加载配置失败: {}", e);
                return 1;
            }
        };

        let profile = match config.profiles.iter().find(|p| p.name == profile_name) {
            Some(p) if p.enabled => p,
            _ => {
                eprintln!("profile \"{}\" 不存在或未启用", profile_name);
                return 1;
            }
        };

        // 启动前刷新链接（确保新项目/记忆同步）
        if let Err(e) = self.link_isolated_home(profile_name) {
            eprintln!("刷新隔离 HOME 链接失败: {}", e);
            return 1;
        }

        let iso_home = self.home_dir_for(profile_name);

        // 启动 claude
        let status = std::process::Command::new("claude")
            .args(passthrough_args)
            .env("HOME", &iso_home)
            .env("USERPROFILE", &iso_home) // Node os.homedir 优先读 USERPROFILE
            .envs(&profile.vars)
            .status();

        match status {
            Ok(s) => s.code().unwrap_or(1),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("未找到 claude 命令，请先安装：npm install -g @anthropic-ai/claude-code");
                127
            }
            Err(e) => {
                eprintln!("启动 claude 失败: {}", e);
                1
            }
        }
    }

    /// 生成包装脚本
    fn generate_script(&self, profile: &Profile) -> String {
        #[cfg(unix)]
        {
            let isolated_home = self.home_dir_for(&profile.name);
            let real_home_str = self.real_home.to_string_lossy();
            let isolated_home_str = isolated_home.to_string_lossy();

            let mut lines = vec![
                "#!/bin/bash".to_string(),
                "# Generated by ccpm — edit in TUI, changes here will be overwritten".to_string(),
                "".to_string(),
                format!("# 保存真实 HOME，隔离 HOME 使 Claude Code 读取 profile 专属 settings.json"),
                format!("REAL_HOME='{}'", real_home_str),
                format!("export HOME='{}'", isolated_home_str),
                "".to_string(),
                format!("# 共享 ~/.claude.json 配置文件"),
                format!("ln -sfn \"$REAL_HOME/.claude.json\" \"$HOME/.claude.json\" 2>/dev/null"),
                format!(""),
                format!("# 共享全部 ~/.claude 内容，只隔离 settings.json"),
                format!("REAL_CLAUDE=\"$REAL_HOME/.claude\""),
                format!("CLAUDE_ISOLATED=\"$HOME/.claude\""),
                format!("mkdir -p \"$CLAUDE_ISOLATED\""),
                format!("for _item in \"$REAL_CLAUDE\"/* \"$REAL_CLAUDE\"/.[!.]*; do"),
                format!("    _name=\"$(basename \"$_item\")\""),
                format!("    [ \"$_name\" = \"settings.json\" ] && continue"),
                format!("    [ -e \"$_item\" ] || continue"),
                format!("    rm -rf \"$CLAUDE_ISOLATED/$_name\" 2>/dev/null"),
                format!("    ln -sfn \"$_item\" \"$CLAUDE_ISOLATED/$_name\" 2>/dev/null"),
                format!("done"),
                format!(""),
                "".to_string(),
            ];

            // export 环境变量（双重保障）
            let mut keys: Vec<&String> = profile.vars.keys().collect();
            keys.sort();
            for key in keys {
                let val = &profile.vars[key];
                let escaped = val.replace('\'', "'\\''");
                lines.push(format!("export {}='{}'", key, escaped));
            }

            lines.push("".to_string());
            lines.push("exec claude \"$@\"".to_string());
            lines.push("".to_string());
            lines.join("\n")
        }

        #[cfg(windows)]
        {
            // Windows 用瘦壳：.cmd 仅仅调用 ccpm.exe __launch 实现所有逻辑。
            // 注意：必须用 \r\n (CRLF) 换行，CMD 遇到 LF-only 会解析异常
            format!(
                "@echo off\r\nrem Generated by ccpm - edit in TUI, changes here will be overwritten\r\n\"%~dp0ccpm.exe\" __launch {} -- %*\r\n",
                profile.name
            )
        }
    }

    /// 删除一个 profile 的包装脚本和隔离 HOME
    pub fn remove(&self, name: &str) -> Result<()> {
        // 删除包装脚本
        let script_path = self.path_for(name);
        if script_path.exists() {
            std::fs::remove_file(&script_path)
                .with_context(|| format!("删除脚本失败: {}", script_path.display()))?;
        }

        // 删除隔离 HOME 目录
        let home_dir = self.home_dir_for(name);
        if home_dir.exists() {
            std::fs::remove_dir_all(&home_dir)
                .with_context(|| format!("删除隔离 HOME 失败: {}", home_dir.display()))?;
        }

        Ok(())
    }

    /// 同步所有 profile
    pub fn sync(&self, config: &Config) -> Result<()> {
        // 确保目录存在
        std::fs::create_dir_all(&self.bin_dir)
            .with_context(|| format!("创建目录失败: {}", self.bin_dir.display()))?;
        std::fs::create_dir_all(&self.homes_dir)
            .with_context(|| format!("创建目录失败: {}", self.homes_dir.display()))?;

        // 为启用的 profile 生成脚本和隔离 HOME
        let mut active_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for p in config.profiles.iter().filter(|p| p.enabled) {
            self.install(p)?;
            // L214 隐藏 bug 修复：必须用 path_for 取最终文件名（带扩展名），否则 Windows 上 active_names 不含 .cmd，清理时把刚创建的包装当成 stale 删掉
            let script_path = self.path_for(&p.name);
            let fname = script_path.file_name().unwrap_or_default().to_string_lossy().to_string();
            active_names.insert(fname);
        }

        // 清理 stale 包装脚本
        if let Ok(entries) = std::fs::read_dir(&self.bin_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() || file_type.is_symlink() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with("ccpm-") && !active_names.contains(name) {
                                let _ = std::fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }

        // 清理 stale 隔离 HOME 目录
        let active_homes: std::collections::HashSet<String> = config.profiles.iter()
            .filter(|p| p.enabled)
            .map(|p| p.name.clone())
            .collect();
        if let Ok(entries) = std::fs::read_dir(&self.homes_dir) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        if !active_homes.contains(name) {
                            let _ = std::fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}