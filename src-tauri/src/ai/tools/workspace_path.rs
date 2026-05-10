use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};

/// 将用户传入的相对路径解析为工作区内的绝对路径，并严格校验是否落在工作区范围内。
///
/// 安全规则：
/// 1. 拒绝空路径
/// 2. 拒绝绝对路径（以 / 或盘符开头）
/// 3. 拒绝包含 ".." 的路径
/// 4. 拒绝包含空组件的路径（如 "a//b"）
/// 5. 拼接后 canonicalize，再检查是否以 workspace_dir 为前缀
/// 6. 符号链接会被 canonicalize 解析为真实路径，若指向工作区外则拒绝
pub fn resolve_workspace_path(workspace_dir: &Path, file_path: &str) -> Result<PathBuf> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err(anyhow!(
            "file_path 不能为空。工作区路径: {}，请使用相对于工作区根目录的路径，如 'AGENTS.md' 或 'docs/plan.md'",
            workspace_dir.display()
        ));
    }

    let p = Path::new(trimmed);

    // 拒绝绝对路径
    if p.is_absolute() {
        return Err(anyhow!(
            "file_path 必须是相对于工作区根目录的路径，不能是绝对路径。工作区路径: {}，正确示例: 'AGENTS.md'、'docs/plan.md'",
            workspace_dir.display()
        ));
    }

    // 拒绝路径遍历
    if trimmed.contains("..") {
        return Err(anyhow!(
            "路径中不允许包含 '..'。工作区路径: {}，请使用相对于工作区根目录的路径，如 'AGENTS.md'",
            workspace_dir.display()
        ));
    }

    // 拒绝空组件（如 a//b）
    for comp in p.components() {
        match comp {
            std::path::Component::Normal(_) => {}
            _ => {
                return Err(anyhow!(
                    "路径包含非法组件: {:?}。工作区路径: {}，请使用相对于工作区根目录的路径",
                    comp,
                    workspace_dir.display()
                ));
            }
        }
    }

    let resolved = workspace_dir.join(trimmed);

    // canonicalize 已存在路径；对于新建文件，fallback 为拼接后的路径
    let (check_path, check_dir) = if resolved.exists() {
        let canon_path = std::fs::canonicalize(&resolved).map_err(|e| anyhow!(
            "无法解析路径 '{}': {}。工作区路径: {}",
            resolved.display(),
            e,
            workspace_dir.display()
        ))?;
        let canon_dir = std::fs::canonicalize(workspace_dir).unwrap_or_else(|_| workspace_dir.to_path_buf());
        (canon_path, canon_dir)
    } else {
        let parent = resolved.parent().ok_or_else(|| anyhow!(
            "无法获取父目录: {}。工作区路径: {}",
            resolved.display(),
            workspace_dir.display()
        ))?;
        let canon_dir = std::fs::canonicalize(workspace_dir).unwrap_or_else(|_| workspace_dir.to_path_buf());
        if parent.exists() {
            let canon_parent = std::fs::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
            (canon_parent.join(resolved.file_name().unwrap_or_default()), canon_dir)
        } else {
            // 父目录不存在（write 工具会创建），用拼接路径做前缀检查
            let fallback = workspace_dir.join(trimmed);
            (fallback, canon_dir)
        }
    };

    if !check_path.starts_with(&check_dir) {
        return Err(anyhow!(
            "只能访问工作区内的文件。工作区路径: {}，你提供的路径解析后: {}，请使用相对于工作区根目录的路径",
            check_dir.display(),
            check_path.display()
        ));
    }

    Ok(resolved)
}

/// 基于扩展名判断是否为文本文件（粗略判断）。
pub fn is_likely_text_file(path: &Path) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext = ext.to_lowercase();
        let binary_exts = [
            "exe", "dll", "so", "dylib", "bin", "o", "a",
            "jpg", "jpeg", "png", "gif", "bmp", "webp", "ico", "svgz",
            "mp3", "mp4", "avi", "mov", "mkv", "webm",
            "pdf", "zip", "gz", "tar", "7z", "rar", "bz2", "xz",
            "doc", "docx", "xls", "xlsx", "ppt", "pptx",
            "db", "sqlite", "sqlite3",
        ];
        if binary_exts.contains(&ext.as_str()) {
            return false;
        }
    }
    true
}
