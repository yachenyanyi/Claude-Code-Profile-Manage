use anyhow::{Context, Result};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use crate::config::profile::Profile;
use crate::config::store::Config;

/// Shell 包装脚本生成器
#[derive(Debug, Clone)]
pub struct Generator {
    bin_dir: PathBuf,
}

impl Generator {
    /// 使用默认路径 ~/.local/bin/
    pub fn new() -> Self {
        let bin_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".local")
            .join("bin");
        Self { bin_dir }
    }

    /// 使用指定路径（用于测试）
    pub fn with_dir(bin_dir: PathBuf) -> Self {
        Self { bin_dir }
    }

    /// 返回某个 profile 对应的脚本路径
    pub fn path_for(&self, name: &str) -> PathBuf {
        self.bin_dir.join(format!("ccp-{}", name))
    }

    /// 将一个 profile 安装为包装脚本
    pub fn install(&self, profile: &Profile) -> Result<()> {
        std::fs::create_dir_all(&self.bin_dir)
            .with_context(|| format!("创建目录失败: {}", self.bin_dir.display()))?;

        let script_path = self.path_for(&profile.name);
        let content = profile.to_script();

        std::fs::write(&script_path, &content)
            .with_context(|| format!("写入脚本失败: {}", script_path.display()))?;

        // chmod +x
        let mut perms = std::fs::metadata(&script_path)
            .with_context(|| format!("读取脚本元数据失败: {}", script_path.display()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&script_path, perms)
            .with_context(|| format!("设置脚本权限失败: {}", script_path.display()))?;

        Ok(())
    }

    /// 删除一个 profile 的包装脚本
    pub fn remove(&self, name: &str) -> Result<()> {
        let path = self.path_for(name);
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("删除脚本失败: {}", path.display()))?;
        }
        Ok(())
    }

    /// 同步所有 profile：为启用的生成脚本，为禁用的删除脚本
    /// 同时清理不再存在于配置中的旧脚本
    pub fn sync(&self, config: &Config) -> Result<()> {
        // 先确保 bin_dir 存在
        std::fs::create_dir_all(&self.bin_dir)
            .with_context(|| format!("创建目录失败: {}", self.bin_dir.display()))?;

        // 为启用的 profile 生成脚本
        let mut active_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for p in config.profiles.iter().filter(|p| p.enabled) {
            self.install(p)?;
            active_names.insert(format!("ccp-{}", p.name));
        }

        // 清理 stale 脚本（不在配置中的 ccp-* 文件）
        if let Ok(entries) = std::fs::read_dir(&self.bin_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() || file_type.is_symlink() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.starts_with("ccp-") && !active_names.contains(name) {
                                let _ = std::fs::remove_file(entry.path());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}