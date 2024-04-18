use anyhow::{anyhow, Result};

/// Get dir name
/// eg: if dir is `.` return `{ACTUAL_DIR_NAME}`
pub fn get_dir_name(dir: &str) -> Result<String> {
    if dir == "." {
        std::env::current_dir()?
            .file_name()
            .ok_or(anyhow!("😣 无法获取文件夹名"))?
            .to_str()
            .ok_or(anyhow!("😣 文件名非法"))
            .map(|s| s.to_string())
    } else {
        Ok(dir.split('/').last().ok_or(anyhow!("😣 文件名解析失败"))?.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dir_name() {
        assert_eq!(get_dir_name(".").unwrap(), "app_dist");
        assert_eq!(get_dir_name("a").unwrap(), "a");
        assert_eq!(get_dir_name("a/b/c").unwrap(), "c");
    }
}
