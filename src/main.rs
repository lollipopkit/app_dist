use std::str::FromStr;

use anyhow::{Ok, Result};
use clap::Parser;
use cli::Ctx;
use strum::VariantNames;
use target::Target;

mod cli;
mod model;
mod target;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = Ctx::parse();
    let mut targets = vec![];
    for target in ctx.targets.iter() {
        if Target::VARIANTS.contains(&target.as_str()) {
            targets.push(Target::from_str(target)?);
        } else {
            eprintln!("😣 Unknown target: {}, it will ne ignored", target);
        }
    }

    for target in targets {
        println!("🔍 {}", target.to_string());
        let entries = target.find_files_without_link().await?;
        let latest = target::get_latest_file(&entries).await?;
        let latest_name = latest
            .file_name()
            .into_string()
            .unwrap_or(format!("{:?}", latest.file_name()));
        println!("🆕 最新的是 {:?}", latest_name);
        target.change_json(latest_name).await?;
        target.rm_old_files(&entries, latest).await?;
        target.link_file(&latest).await?;
        println!("🎉 已完成\n")
    }
    Ok(())
}
