use std::io::Write;

use anyhow::{anyhow, Ok, Result};
use difference::{Changeset, Difference};
use core::result::Result::Ok as Okk;
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString, VariantNames};
use tokio::fs::{self, DirEntry};

const VERSION_REG: &str = r"\d+";
lazy_static! {
    static ref VERSION_REGEX: Regex = Regex::new(VERSION_REG).unwrap();
}

#[derive(Debug, EnumString, Display, VariantNames, AsRefStr)]
pub enum Target {
    #[strum(serialize = "android")]
    Android,
    #[strum(serialize = "ios")]
    Ios,
    #[strum(serialize = "mac")]
    Mac,
    #[strum(serialize = "linux")]
    Linux,
    #[strum(serialize = "windows")]
    Windows,
}

impl Target {
    fn suffix(&self) -> &str {
        match self {
            Self::Android => "apk",
            Self::Ios => "ipa",
            Self::Mac => "app.zip",
            Self::Linux => "AppImage",
            Self::Windows => "win.zip",
        }
    }

    pub async fn find_files_without_link(&self) -> Result<Vec<DirEntry>> {
        let suffix = self.suffix();
        let mut entries = fs::read_dir(".").await?;
        let mut result = vec![];
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some(suffix) {
                if let Okk(metadata) = fs::symlink_metadata(&path).await {
                    if !metadata.file_type().is_symlink() {
                        result.push(entry);
                    }
                }
            }
        }
        Ok(result)
    }

    pub async fn change_json(&self, file_path: String) -> Result<()> {
        let update_content = fs::read_to_string("update.json").await?;
        let mut obj: Value = serde_json::from_str(&update_content)?;
        let file_name = file_path.split('/').last().ok_or(anyhow!("No file name"))?;
        let target_name = self.as_ref();

        // 改变链接
        match self {
            Target::Android | Target::Linux | Target::Windows => {
                obj["url"][target_name] = file_name.into()
            }
            // Pass
            _ => (),
        }

        // 改变版本号
        // 先正则匹配文件名，如果失败，则请求输入
        let version: u32 = match VERSION_REGEX.find(file_name) {
            Some(version) => version.as_str().parse()?,
            None => ask_input("🔢 请输入版本号：")?.parse()?,
        };
        obj["build"]["last"][target_name] = version.into();

        // 显示差异，要求确认
        let new_content = serde_json::to_string_pretty(&obj)?;
        let changes = Changeset::new(&update_content, &new_content, "\n");
        for change in changes.diffs {
            match change {
                Difference::Add(ref x) => println!("+ {}", x),
                Difference::Rem(ref x) => println!("- {}", x),
                Difference::Same(_) => {},
            }
        }
        let resume = ask_resume(Some("📃 是否更新 update.json？"), true)?;
        if resume {
            fs::write("update.json", new_content).await?;
        }
        Ok(())
    }

    pub async fn rm_old_files(&self, entries: &Vec<DirEntry>, latest: &DirEntry) -> Result<()> {
        if entries.len() == 1 {
            println!("📃 没有需要删除的旧文件～");
            return Ok(());
        }
        let paths = entries
            .iter()
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        let prompt = format!(
            "📃 共计 {} 个旧文件 {:?}, 是否删除？",
            paths.len(),
            paths,
        );
        if !ask_resume(Some(&prompt), false)? {
            return Ok(());
        }
        for entry in entries {
            if entry.path() != latest.path() {
                fs::remove_file(entry.path()).await?;
            }
        }
        Ok(())
    }

    pub async fn link_file(&self, latest: &DirEntry) -> Result<()> {
        let target = format!("latest.{}", self.suffix());
        set_link(latest, &target).await?;
        Ok(())
    }
}

pub async fn get_latest_file<'a>(entries: &'a Vec<DirEntry>) -> Result<&'a DirEntry> {
    let mut latest = entries
        .first()
        .ok_or_else(|| anyhow::anyhow!("No file found"))?;
    let mut latest_time = latest.metadata().await?.modified()?;
    for entry in entries.iter().skip(1) {
        let time = entry.metadata().await?.modified()?;
        if time > latest_time {
            latest = entry;
            latest_time = time;
        }
    }
    Ok(latest)
}

fn ask_resume(prompt: Option<&str>, default_true: bool) -> Result<bool> {
    let mut input = String::new();
    loop {
        print!(
            "{} {}",
            prompt.unwrap_or("❓ 是否继续？"),
            if default_true { "[Y/n]" } else { "[y/N]" }
        );
        std::io::stdout().flush()?;
        std::io::stdin().read_line(&mut input)?;
        match input.to_lowercase().trim() {
            "y" => return Ok(true),
            "n" => return Ok(false),
            _ => {
                eprintln!("Invalid input: {}", input);
                input.clear();
            }
        }
    }
}

fn ask_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input)
}

async fn set_link(src: &DirEntry, target: &str) -> Result<()> {
    let src = src.path();
    let same_file = match fs::read_link(target).await {
        Okk(link) => link == src,
        Err(_) => false,
    };
    if same_file {
        println!("🔗 链接与目标相同，跳过：{}", src.display());
    } else {
        fs::remove_file(target).await?;
        fs::symlink(&src, target).await?;
        println!("🔗 链接成功：{} -> {}", target, src.display());
    }
    Ok(())
}
