# war3-manager

魔兽争霸 III 地图归档管理工具。它把散落的地图与压缩包整理成内容寻址的数据集，
维护可搜索目录，并完成 Hugging Face 数据集的增量发布与校验。

- 公开站点：[war3-archive/war3-maps](https://github.com/war3-archive/war3-maps)
- 数据集：[magicwenli/war3-maps](https://huggingface.co/datasets/magicwenli/war3-maps)

## 能做什么

- 递归扫描地图、战役包与常见压缩包
- 按 SHA-256 去重并生成地图目录
- 增量合并新批次，维护来源与栏目归属
- 重扫元数据、版本、封面和第三方脚本修改
- 上传并验证 Hugging Face 数据集

## 快速开始

```bash
cargo build --profile catalog -p war3-manager-cli

cargo run --profile catalog -p war3-manager-cli -- build-catalog \
  /path/to/downloads \
  --out-dir /path/to/war3-maps-dataset \
  --hf-repo magicwenli/war3-maps
```

查看全部命令：

```bash
cargo run -p war3-manager-cli -- help
```

## License

MIT
