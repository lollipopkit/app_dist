use std::{io::Write, path::Path};

use anyhow::{anyhow, Ok, Result};
use core::result::Result::Ok as Okk;
use difference::{Changeset, Difference};
use lazy_static::lazy_static;
use regex::Regex;
use serde_json::Value;
use strum::{AsRefStr, Display, EnumString, VariantNames};
use tokio::fs::{self, DirEntry};

use crate::{arch::Arch, cli::Ctx, print_flush, util};

const VERSION_REG: &str = r"\d+";
lazy_static! {
    static ref VERSION_REGEX: Regex = Regex::new(VERSION_REG).unwrap();
}

const UPDATE_FILE_NAME: &str = "update.json";
const UPDATE_FILE_BAK: &str = const_format::concatcp!(UPDATE_FILE_NAME, ".bak");

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

    pub async fn find_files_without_link(&self, dir: &str) -> Result<Vec<DirEntry>> {
        let suffix = self.suffix();
        let mut entries = fs::read_dir(dir).await?;
        let mut result = vec![];
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file()
                && path
                    .to_str()
                    .ok_or_else(|| anyhow!("😣 未能解析路径：{:?}", path))?
                    .ends_with(suffix)
            {
                if let Okk(metadata) = fs::symlink_metadata(&path).await {
                    if !metadata.file_type().is_symlink() {
                        result.push(entry);
                    }
                }
            }
        }
        Ok(result)
    }

    // 如果最新版本号与当前最新文件一致，返回 true
    pub async fn change_json(&self, file_path: &String, ctx: &Ctx) -> Result<bool> {
        if !ctx.change_json {
            println!("📃 根据参数，跳过");
            return Ok(false);
        }
        let update_path = Path::new(&ctx.dir);
        let update_path = update_path.join(UPDATE_FILE_NAME);
        let update_content = fs::read_to_string(&update_path).await?;
        let mut obj: Value = serde_json::from_str(&update_content)?;
        let target_name = self.as_ref();

        // 改变版本号
        // 先正则匹配文件名，如果失败，则请求输入
        let version: u32 = match VERSION_REGEX.find(&file_path) {
            Some(version) => version.as_str().parse()?,
            None => ask_input("🔢 请输入版本号：")?.parse()?,
        };
        let last_version = obj["build"]["last"][target_name].as_u64().unwrap_or(0) as u32;
        if last_version == version {
            println!("📃 版本号相同，跳过：{}\n", version);
            return Ok(true);
        }
        obj["build"]["last"][target_name] = version.into();

        let arch = if let Some(arch) = Arch::from_str(file_path) {
            arch.to_string()
        } else {
            ask_input("🔢 请输入架构：")?
        };

        // 改变链接
        match self {
            Target::Android | Target::Linux | Target::Windows => {
                let url = format!(
                    "https://cdn.lolli.tech/{}/{}",
                    util::get_dir_name(&ctx.dir)?,
                    &file_path
                );
                obj["urls"][target_name][arch] = url.into();
            }
            Target::Ios | Target::Mac => println!("📌 跳过更新链接"),
        }

        // 显示差异，要求确认
        let new_content = serde_json::to_string_pretty(&obj)?;
        let old_lines: Vec<&str> = update_content.split('\n').collect();
        let new_lines: Vec<&str> = new_content.split('\n').collect();
        let mut diffs = Vec::new();
        for (line1, line2) in old_lines.iter().zip(new_lines.iter()) {
            let changeset = Changeset::new(line1, line2, "\n");
            diffs.push(changeset);
        }
        for diff in diffs {
            for change in diff.diffs {
                match change {
                    Difference::Same(_) => (),
                    Difference::Add(ref x) => println!("+  {}", x),
                    Difference::Rem(ref x) => println!("-  {}", x),
                }
            }
        }
        let resume = ask_resume(Some("📃 是否更新？"), true)?;
        if resume {
            // 先备份
            let bak_path = Path::new(&ctx.dir).join(UPDATE_FILE_BAK);
            fs::copy(&update_path, bak_path).await?;
            fs::write(&update_path, new_content).await?;
        }
        Ok(false)
    }

    pub async fn rm_old_files(
        &self,
        entries: &Vec<DirEntry>,
        latest: &DirEntry,
        rm: bool,
    ) -> Result<()> {
        if !rm {
            println!("📃 根据参数，跳过");
            return Ok(());
        }
        if entries.len() == 1 {
            println!("📃 没有需要删除的旧文件～");
            return Ok(());
        }
        let paths = entries.iter().map(|entry| entry.path()).collect::<Vec<_>>();
        let prompt = format!("📃 共计 {} 个旧文件, 是否删除？", paths.len());
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

    pub async fn link_file(&self, latest: &DirEntry, ctx: &Ctx) -> Result<()> {
        if !ctx.link {
            println!("📃 根据参数，跳过");
            return Ok(());
        }
        let target = format!("{}/latest.{}", &ctx.dir, self.suffix());
        set_link(latest, &target).await?;
        Ok(())
    }
}

pub async fn get_latest_file<'a>(entries: &'a Vec<DirEntry>) -> Result<&'a DirEntry> {
    let mut latest = entries
        .first()
        .ok_or_else(|| anyhow::anyhow!("😣 文件列表为空"))?;
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
        print_flush!(
            "{} {}",
            prompt.unwrap_or("❓ 是否继续？"),
            if default_true { "[Y/n] " } else { "[y/N] " }
        );
        std::io::stdin().read_line(&mut input)?;
        if input == "\n" {
            return Ok(default_true);
        }
        match input.to_lowercase().trim() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => {
                eprintln!("Invalid input: {}", input);
                input.clear();
            }
        }
    }
}

fn ask_input(prompt: &str) -> Result<String> {
    print_flush!("{}", prompt);
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
        let resume = ask_resume(Some("🔗 是否创建链接？"), true)?;
        if !resume {
            return Ok(());
        }
        let _ = fs::remove_file(target).await;
        if let Some(src_name) = &src.file_name() {
            fs::symlink(src_name, target).await?;
        } else {
            eprintln!("😣 未能解析文件名：{}", src.display());
        }
    }
    Ok(())
}
