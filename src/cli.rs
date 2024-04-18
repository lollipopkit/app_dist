use clap::{arg, command, Parser};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct Ctx {
    #[arg(short, long, help = "是否删除旧的安装包", default_value_t = true)]
    pub rm_old_files: bool,

    // 所有目标
    pub targets: Vec<String>,

    #[arg(short, long, help = "是否创建软链接", default_value_t = true)]
    pub link: bool,

    #[arg(short, long, help = "是否修改 json 文件", default_value_t = true)]
    pub change_json: bool,

    #[arg(short, long, help = "指定文件夹", default_value = ".")]
    pub dir: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let ctx = Ctx::parse_from(&["-d", "test_dir", "android"]);
        assert_eq!(ctx.rm_old_files, true);
        assert_eq!(ctx.targets, vec!["android"]);
        assert_eq!(ctx.link, true);
        assert_eq!(ctx.change_json, true);
        assert_eq!(ctx.dir, "test_dir");
    }
}
