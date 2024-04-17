#!/usr/bin/env dart

import 'dart:convert';
import 'dart:io';

const scriptName = 'release_cfg.dart';
const helpMsg = '''
eg:
  $scriptName mac,apk -> 清理 mac/apk，改变版本号
  $scriptName -> 清理所有，改变版本号
''';
const encoder = JsonEncoder.withIndent('\t');

void main(List<String> args) async {
  List<Target> targets = [];
  if (args.length == 1) {
    final arg = args[0];
    if (arg == 'all') {
      targets = Target.values;
    } else {
      for (final target in arg.split(',')) {
        for (int idx = 0; idx < Target.values.length; idx++) {
          final t = Target.values[idx];
          if (t.name == target) {
            targets.add(t);
            break;
          }
          if (idx == Target.values.length - 1) {
            print('❌ 不支持的目标：$target');
            return;
          }
        }
      }
    }
  } else {
    printHelp();
    return;
  }

  for (final target in targets) {
    await target.tidy();
  }
}

void printHelp() {
  print('❌ 参数错误\n' + helpMsg);
}

enum Target {
  android._('apk'),
  mac._('app.zip'),
  ios._('ipa'),
  linux._('AppImage'),
  windows._('win.zip'),
  ;

  final String suffix;

  const Target._(this.suffix);

  Stream<FileSystemEntity> get findFilesWithoutLink async* {
    await for (final file in Directory.current.list()) {
      if (!file.path.endsWith(suffix)) continue;
      if (!await FileSystemEntity.isLink(file.path)) {
        yield file;
      }
    }
  }

  Future<FileSystemEntity?> getLatest(List<FileSystemEntity> files) async {
    if (files.isEmpty) {
      print('⚠️ 没有找到任何文件～\n');
      return null;
    }
    var latest = files[0];
    var latestTime = (await latest.stat()).modified;
    for (final file in files) {
      final time = (await file.stat()).modified;
      if (time.isAfter(latestTime)) {
        latest = file;
        latestTime = time;
      }
    }
    print('🆕 最新的是 ${latest.path}');
    return latest;
  }

  Link get latestLink => Link('latest.$suffix');

  Future<void> rmOldFiles(
    List<FileSystemEntity> files,
    FileSystemEntity latest,
  ) async {
    if (files.length <= 1) {
      print('📃 没有需要删除的旧文件～');
      return;
    }
    files.remove(latest);
    print('📃 共计 ${files.length} 个旧文件：${files.map((e) => e.path)}');
    askResume(
      prompt: '📃 是否删除？',
      onTrue: () async {
        for (var file in files) {
          await file.delete();
        }
      },
    );
  }

  Future<void> changeJson(String filepath) async {
    final file = File('update.json');
    final content = await file.readAsString();
    await File('update.json.bak').writeAsString(content);
    final obj = json.decode(content);
    filepath = filepath.split('/').last;
    final curDirName = Directory.current.path.split('/').last;
    switch (this) {
      case Target.android:
      case Target.windows:
      case Target.linux:
        obj['url'][name] = 'https://res.lolli.tech/$curDirName/' + filepath;
        break;
      case Target.mac:
      case Target.ios:
        // 跳过 iOS / macOS，因为它们的下载链接是固定的
        break;
    }

    // 改变版本号
    final versionStr = filepath.allMatches(r'\d+').first.group(0);
    var version = int.tryParse(versionStr ?? '');
    if (version == null) {
      final input = askInput(
        prompt: '❓ 请输入版本号：',
        defaultInput: obj['build']['last'][name].toString(),
      );
      version = int.tryParse(input);
      if (version == null) {
        print('❌ 版本号错误：$input');
        return;
      }
    }
    obj['build']['last'][name] = version;

    final result = encoder.convert(obj);
    askResume(
      prompt: '📃 是否更新 update.json？',
      onTrue: () async {
        await file.writeAsString(result);
      },
    );
  }

  Future<void> tidy() async {
    print('[${name.toUpperCase()}]');
    final files = await findFilesWithoutLink.toList();
    final latest = await getLatest(files);
    if (latest == null) {
      return;
    }

    await changeJson(latest.path);
    await rmOldFiles(files, latest);
    await setLink(latest, latestLink);
    print('🎉 已完成\n');
  }
}

/// Return [true] if [stdin.readLineSync] is not 'n'
///
/// Only use it in sub func instead of [Target.tidy]
void askResume({
  String? prompt = '❓ 是否继续？',
  void Function()? onTrue,
  void Function()? onFalse,
  bool defaultTrue = true,
}) {
  stdout.write('$prompt ${defaultTrue ? "[Y/n]" : "[y/N]"} ');
  final defaultHandler = defaultTrue ? onTrue : onFalse;
  return switch (stdin.readLineSync()?.toLowerCase()) {
    'y' || 'yes' => onTrue?.call(),
    'n' || 'no' => onFalse?.call(),
    _ => defaultHandler?.call(),
  };
}

String askInput({
  String? prompt,
  String? defaultInput,
}) {
  stdout.write('$prompt ${defaultInput == null ? '' : '[$defaultInput]'} ');
  final input = stdin.readLineSync();
  return input?.isEmpty == true ? defaultInput ?? '' : input!;
}

Future<void> setLink(FileSystemEntity src, FileSystemEntity target) async {
  final link = Link(target.path);

  /// 判断 [link] 是否和 [target] 是同一个文件
  if (await link.exists() && await link.target() == src.path) {
    print('🔗 链接与目标相同，跳过：${link.path} ');
    return;
  }
  askResume(
    prompt: '🔗 是否创建链接 ${target.path} ？',
    onTrue: () async {
      if (await link.exists()) {
        await link.delete();
      }
      await link.create(src.path);
    },
  );
}