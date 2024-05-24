use std::str::FromStr;

use anyhow::{Ok, Result};
use clap::Parser;
use cli::Ctx;
use strum::VariantNames;
use target::Target;

mod arch;
mod cli;
mod target;
mod macros;
mod util;

#[tokio::main]
async fn main() -> Result<()> {
    let ctx = Ctx::parse();
    let mut targets = vec![];
    for target in ctx.targets.iter() {
        if Target::VARIANTS.contains(&target.as_str()) {
            targets.push(Target::from_str(target)?);
        } else {
            eprintln!("😣 未知目标: {}, 将被忽略", target);
        }
    }

    for target in targets {
        println!("[{}]", target.to_string().to_uppercase());
        let entries = target.find_files_without_link(&ctx.dir).await?;
        let latest = target::get_latest_file(&entries).await?;
        let latest_name = latest
            .file_name()
            .into_string()
            .unwrap_or(format!("{:?}", latest.file_name()));
        let skip = target.change_json(&latest_name, &ctx).await?;
        if skip {
            continue;
        }
        println!("🆕 最新的是 {:?}", &latest_name);
        target.rm_old_files(&entries, latest, ctx.rm_old_files).await?;
        target.link_file(&latest, &ctx).await?;
        println!("🎉 已完成\n")
    }
    Ok(())
}
