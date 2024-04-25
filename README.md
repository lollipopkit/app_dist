## app_dist
A cli tool to distribute apps by configuring static files (install pkgs & update info json).

### Usage
```shell
Usage: app_dist [OPTIONS] [TARGETS]...

Arguments:
  [TARGETS]...  

Options:
  -r, --rm-old-files  是否删除旧的安装包
  -l, --link          是否创建软链接
  -c, --change-json   是否修改 json 文件
  -d, --dir <DIR>     指定文件夹 [default: .]
  -h, --help          Print help
  -V, --version       Print version
```