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

uv run --project deploy war3-deploy apply-rescan \
  /path/to/war3-maps-dataset rescan.jsonl

uv run --project deploy war3-deploy export-covers \
  /path/to/war3-maps-dataset
```

也可以使用 `scan-versions` 或 `scan-mods` 只刷新对应字段，再分别运行
`apply-versions` 或 `apply-mods`。

## 运维工具

`deploy/` 是一个由 [uv](https://docs.astral.sh/uv/) 管理的 Python 项目
（`war3-deploy`），依赖锁在 `deploy/uv.lock`。`uv run` 会按需创建并同步虚拟环境，
不需要手动装依赖：

```bash
uv run --project deploy war3-deploy --help
# 或者
just deploy --help
```

| 子命令 | 作用 |
|--------|------|
| `merge` | 按 SHA-256 增量合并新批次 |
| `apply-rescan` | 回填完整重扫结果 |
| `apply-mods` | 回填第三方脚本修改检测结果 |
| `apply-versions` | 回填 w3i 版本字段 |
| `classify-tags` | 使用本地模型或远程文本 API 生成可审核、可续跑的玩法/系列/题材标签候选 |
| `export-covers` | 导出 WebP 封面并写入 `cover_path`、`cover_url` |
| `upload` | 校验并上传内容寻址数据集 |
| `verify` | 发布后检查栏目、失败记录、封面与下载链接 |

长流程的进度打在 stderr（终端里就地刷新，重定向到文件时每步一行），JSON 报告打在
stdout，因此 `... | jq` 只会读到报告。

### AI 标签候选

`classify-tags` 不会覆盖历史 `collection` 或 `category`。它会去掉标题和简介中的
Warcraft III 颜色/换行控制码，向本机或远程 OpenAI 兼容模型请求多标签，并在每批完成后
写入 `catalog/tag-candidates.jsonl`；重跑会跳过同一 schema 的已有 SHA-256，可安全续跑。
默认会关闭 OpenRouter 的 reasoning，截断则将该批自动拆分重试。基础词表外的新标签只允许
使用 `玩法:`、`系列:`、`题材:` 命名空间，每图最多三个，并会在最终报告的
`taxonomy_extensions` 中汇总；用 `--no-new-tags` 可以强制固定词表。
远程 API 可加 `--workers 8` 并发请求，候选检查点仍串行写入，不会发生文件竞争。

Apple Silicon 上可用 MLX 启动默认的小模型：

```bash
uv tool install mlx-lm
mlx_lm.server --model mlx-community/Qwen3-4B-Instruct-2507-4bit --port 8080

uv run --project deploy war3-deploy classify-tags /path/to/war3-maps-dataset
```

完成后先抽样审阅候选，再明确写入目录：

```bash
uv run --project deploy war3-deploy classify-tags /path/to/war3-maps-dataset --apply
```

`--apply` 要求候选数与目录地图数完全相同，并新增 `tags`、`tag_confidence`、
`tag_evidence`、`tag_schema_version`。上传带这些字段的目录前，必须把更新后的
`deploy/hf/DATASET_README.template.md` 复制为数据集根目录的 `README.md`，否则 Hub
viewer 会忽略新字段。

地图修改检测的字段和规则见 [MOD_DETECTION.md](MOD_DETECTION.md)。

## 发布顺序

```text
merge → export-covers → upload → verify
```

必须在上传前导出封面，否则目录里的 `cover_url` 会指向数据集中尚不存在的文件。

首次上传前，按实际来源、许可证和联系方式补全
`deploy/hf/DATASET_README.template.md`，并将其保存为数据集根目录的 `README.md`。

该模板的 YAML front matter 决定 Hub 上的 Dataset Viewer 显示什么：`configs`
把预览锁定在 `catalog/maps.jsonl`（不写它时，Hub 会把 `covers/` 当成
imagefolder 自动推断，预览只剩 image + sha 前缀 label 两列，而且上万个文件的
parquet 转换跑不完，viewer/search/filter 全部不可用）；`dataset_info.features`
显式钉死目录的字段类型，`build_version`（仅 5 条非空）和 `modification`（仅
2544 条非空）这类稀疏列不再依赖分块推断。**目录新增字段时必须同步补进
`features`**，否则新字段会被 `unexpected_field_behavior=ignore` 静默丢掉。改完
模板记得重新复制到数据集根目录，再上传。

认证走 `hf auth login`（命令会读取它存下的 token）；也可以用 `HF_TOKEN=hf_...`
或 `--token` 覆盖。`--dry-run` 只做本地校验，不需要认证。

先做本地校验，再正式上传：

```bash
uv run --project deploy war3-deploy upload \
  /path/to/war3-maps-dataset \
  --repo-id magicwenli/war3-maps \
  --dry-run

uv run --project deploy war3-deploy upload \
  /path/to/war3-maps-dataset \
  --repo-id magicwenli/war3-maps
```

脚本默认创建公开数据集，并检查 SHA-256 唯一性、对象是否存在、文件大小和内容哈希。
重复上传同一目录是安全的；确认本地目录已经独立校验后，可用 `--skip-hash-check`
跳过耗时的全量内容哈希。

目录站由独立的
[war3-archive/war3-maps](https://github.com/war3-archive/war3-maps) 项目维护，
本仓库只负责生成并发布它消费的 catalog 与对象文件。
