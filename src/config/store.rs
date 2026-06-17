use super::profile::Profile;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// 顶层配置结构
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "profiles")]
    pub profiles: Vec<Profile>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),
        }
    }
}

/// 配置存储
#[derive(Debug, Clone)]
pub struct Store {
    path: PathBuf,
}

impl Store {
    /// 使用默认路径创建 Store：~/.config/ccpm/profiles.toml
    pub fn new() -> Self {
        let path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("ccpm")
            .join("profiles.toml");
        Self { path }
    }

    /// 使用指定路径创建 Store（用于测试）
    pub fn with_path(path: PathBuf) -> Self {
        Self { path }
    }

    /// 返回配置文件路径
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 从磁盘加载配置，文件不存在时返回空配置
    pub fn load(&self) -> Result<Config> {
        if !self.path.exists() {
            return Ok(Config::default());
        }
        let content = std::fs::read_to_string(&self.path)
            .with_context(|| format!("读取配置文件失败: {}", self.path.display()))?;
        if content.trim().is_empty() {
            return Ok(Config::default());
        }
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("解析配置文件失败: {}", self.path.display()))?;
        Ok(config)
    }

    /// 保存配置到磁盘，自动创建父目录
    pub fn save(&self, config: &Config) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("创建配置目录失败: {}", parent.display()))?;
        }
        let content =
            toml::to_string_pretty(config).with_context(|| "序列化配置失败")?;
        std::fs::write(&self.path, &content)
            .with_context(|| format!("写入配置文件失败: {}", self.path.display()))?;
        std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600))
            .with_context(|| format!("设置配置文件权限失败: {}", self.path.display()))?;
        Ok(())
    }
}
