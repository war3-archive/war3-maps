# 操作与发布流程

本文说明 `war3-manager` 的命令、数据集布局、重扫回填和发布步骤。

## 构建

```bash
cargo build --profile catalog -p war3-manager-cli
# 或
just build-catalog
```

`catalog` profile 保留 release 优化，但使用 `panic = unwind`。扫描器会为每个输入建立
独立的错误边界，因此一张损坏地图只会产生一条 `metadata_error`，不会中断整个批次。

## CLI 命令

```text
build-catalog  扫描地图与压缩包，按 SHA-256 去重，生成静态站点目录
scan-versions  重读已有数据集的 w3i 版本字段，输出 JSONL
scan-mods      重扫已有数据集的第三方脚本修改，输出 JSONL
rescan         重读全部元数据并补齐缺失封面，输出 JSONL
```

运行 `war3-manager <command> --help` 可查看每个命令的完整参数。

## 建立数据集

```bash
cargo run --profile catalog -p war3-manager-cli -- build-catalog \
  /path/to/downloads \
  --out-dir /path/to/war3-maps-dataset \
  --hf-repo magicwenli/war3-maps
```

扫描不会修改输入文件。支持松散的 `.w3x`、`.w3m`、`.w3n`，以及 `.zip`、`.rar`、
`.7z` 和常见 tar 包中的地图成员。压缩包扫描使用系统 `bsdtar`；传入
`--no-archives` 可只处理松散文件。

输出布局：

```text
war3-maps-dataset/
  objects/55/55d87b….w3x    # 每个 SHA-256 一份
  covers/55/55d87b….webp    # 封面缩略图
  catalog/maps.json         # 站点使用的目录
  catalog/maps.jsonl        # 每行一条记录
  catalog/failures.json     # 无法读取的输入
```

重复文件的来源保存在 `source_paths`。地图名中的 Warcraft 颜色/换行代码和未解析的
`TRIGSTR_*` 会被清理，同时保留原始文件名。战役包当前使用
`content_type: "campaign"` 和 `parse_status: "metadata_unavailable"`，可通过文件名检索。

### `parse_status`

| 值 | 含义 |
|----|------|
| `ok` | 档案正常打开，`war3map.w3i` 按名读取 |
| `carved` | 档案打不开（哈希表/块表被破坏），但从原始扇区数据里雕出了一份 `w3i`；`parse_error` 仍记录档案本身的失败原因 |
| `metadata_error` | 档案打不开，扇区里也找不到可信的 `w3i` |
| `metadata_unavailable` | 战役包（`.w3n`），当前不解析元数据 |

`carved` 记录的元数据可信但不权威：雕取只看扇区数据，没有表能说明哪个扇区是“活”的，
重存过的地图可能留下旧版本的 `w3i`。因此 `name_source` 用 `w3i_carved` 与按名读取的
`w3i` 区分开，且排在明文 `hm3w` 标题之后——数据集里 224 条 `carved` 记录中有 26 条
两者不一致，而文件实际发布用的是 `hm3w` 里的标题。

## 重扫与回填

解析能力改善后，无需重建数据集。以下命令会重读对象、按 SHA-256 回填元数据，并保留
已有栏目与来源信息：

```bash
just rescan /path/to/war3-maps-dataset
```

对应的完整步骤是：

```bash
cargo run --profile catalog -p war3-manager-cli -- \
  rescan /path/to/war3-maps-dataset -o rescan.jsonl

python3 deploy/apply_rescan.py \
  /path/to/war3-maps-dataset rescan.jsonl

python3 deploy/export_covers.py \
  /path/to/war3-maps-dataset
```

也可以使用 `scan-versions` 或 `scan-mods` 只刷新对应字段，再分别运行
`apply_versions.py` 或 `apply_mods.py`。

## 运维脚本

- `deploy/merge_dataset.py`：按 SHA-256 增量合并新批次
- `deploy/apply_rescan.py`：回填完整重扫结果
- `deploy/apply_mods.py`：回填第三方脚本修改检测结果
- `deploy/apply_versions.py`：回填 w3i 版本字段
- `deploy/export_covers.py`：导出 WebP 封面并写入 `cover_path`、`cover_url`
- `deploy/hf/upload_dataset.py`：校验并上传内容寻址数据集
- `deploy/verify_final.py`：发布后检查栏目、失败记录、封面与下载链接

地图修改检测的字段和规则见 [MOD_DETECTION.md](MOD_DETECTION.md)。

## 发布顺序

```text
merge_dataset.py
  → export_covers.py
  → hf/upload_dataset.py
  → verify_final.py
```

必须在上传前导出封面，否则目录里的 `cover_url` 会指向数据集中尚不存在的文件。

首次上传前，按实际来源、许可证和联系方式补全
`deploy/hf/DATASET_README.template.md`，并将其保存为数据集根目录的 `README.md`。
然后安装上传脚本依赖：

```bash
python3 -m venv .venv-hf
.venv-hf/bin/pip install -r deploy/hf/requirements.txt
```

认证走 `hf auth login`（脚本会读取它存下的 token）；也可以用 `HF_TOKEN=hf_...`
或 `--token` 覆盖。`--dry-run` 只做本地校验，不需要认证。

先做本地校验，再正式上传：

```bash
.venv-hf/bin/python deploy/hf/upload_dataset.py \
  /path/to/war3-maps-dataset \
  --repo-id magicwenli/war3-maps \
  --dry-run

.venv-hf/bin/python deploy/hf/upload_dataset.py \
  /path/to/war3-maps-dataset \
  --repo-id magicwenli/war3-maps
```

脚本默认创建公开数据集，并检查 SHA-256 唯一性、对象是否存在、文件大小和内容哈希。
重复上传同一目录是安全的；确认本地目录已经独立校验后，可用 `--skip-hash-check`
跳过耗时的全量内容哈希。

目录站由独立的
[war3-archive/war3-maps](https://github.com/war3-archive/war3-maps) 项目维护，
本仓库只负责生成并发布它消费的 catalog 与对象文件。
