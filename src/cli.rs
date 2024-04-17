use clap::{arg, command, Parser};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Ctx {
    #[arg(short, long, help = "是否删除旧的安装包")]
    pub rm_old_files: bool,

    // 所有目标
    pub targets: Vec<String>,
}