# 魔兽争霸 III 地图索引

免费、可搜索、支持单文件下载的魔兽争霸 III 地图资料库。

- [打开地图搜索网站](https://war3-archive.github.io/war3-maps/)
- [浏览数据仓库](https://huggingface.co/datasets/magicwenli/war3-maps)

## 收录范围

收录 17 个栏目的社区地图与战役，包含作者、简介、玩家人数、版本与封面缩略图等信息。同一份文件按内容去重，只保存一次。平台氪金图不在收录范围内。

## 使用

网站支持按名称、作者、分类搜索，并为每张地图提供独立下载链接；文件均记录内容指纹，便于核对完整性。

## 致谢

特别感谢哔哩哔哩 UP 主 [关先生丶的游戏实况](https://space.bilibili.com/2534568)，本库相当一部分经典老图来自他的整理与分享。

## 仓库结构

网站、解析工具链和数据管线都在这个仓库里，五个 crate 共用一个版本号，由 `cargo release` 统一推进。

| 目录 | 内容 | 文档 |
|---|---|---|
| `crates/war3-mpq` | MPQ 读取，针对被保护的地图加固 | [docs/MPQ.md](docs/MPQ.md)、[docs/PROTECTED_MAPS.md](docs/PROTECTED_MAPS.md) |
| `crates/war3parser` | 地图格式解析（w3i/wts/blp…），另发 npm wasm 包 | [docs/PARSER.md](docs/PARSER.md) |
| `crates/war3parser-cli` | 解析器命令行 | [docs/PARSER.md](docs/PARSER.md) |
| `crates/war3parser-wasm` | wasm 绑定，站点在浏览器里解析地图用 | [docs/PARSER.md](docs/PARSER.md) |
| `crates/war3-manager-cli` | 编目、数据集与发布流程 | [docs/MANAGER.md](docs/MANAGER.md)、[docs/OPERATIONS.md](docs/OPERATIONS.md) |
| `src`、`public`、`scripts` | Astro 站点 | — |
| `deploy` | 数据集导出与 Hugging Face 同步 | [docs/OPERATIONS.md](docs/OPERATIONS.md) |

常用命令见 `just --list`：`just test`、`just rescan <dataset>`、`just release <level>`。

## 权利与下架

地图版权归各自作者所有，收录不代表取得所有权或额外授权。需要补充署名、修正信息或申请移除时，请在本仓库提交 Issue，并附上文件指纹与必要说明。
