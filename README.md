# war3-manager

魔兽争霸 III 地图归档的管理工具：把散落的地图包整理成内容寻址的数据集，维护目录，
发布到 Hugging Face 与静态站点。

解析本身不在这里。地图格式、MPQ 读取、修改检测都由上游
[war3parser](https://github.com/wesleyel/war3parser) 提供，本仓库以依赖方式使用它。
底层 MPQ 修复在 [war3-archive/mpq-rust](https://github.com/war3-archive/mpq-rust)。

- 公开站点：[war3-archive/war3-maps](https://github.com/war3-archive/war3-maps)
- 数据集：[magicwenli/war3-maps](https://huggingface.co/datasets/magicwenli/war3-maps)

## 职责边界

| 位置 | 归属 |
|---|---|
| MPQ 底层读取 | mpq-rust |
| 地图格式解析、通用 API、WASM 绑定 | war3parser |
| 目录 schema、栏目归属、HF URL | war3-manager |
| `deploy/**`、数据集运维文档 | war3-manager |

一次提交不要同时改上游解析和这里的目录逻辑：解析改动走 war3parser 的 PR，
本仓库只调用新 API。PR 分支从 `upstream/main` 开，不要从本仓库的 `main` 开：

```bash
git fetch upstream
git worktree add -b pr/<topic> ../war3parser-pr-<topic> upstream/main
```

定期把上游合并进来：

```bash
git fetch upstream && git merge --no-ff upstream/main
```

## 命令

```bash
cargo build --profile catalog -p war3-manager-cli   # 或 just build-catalog
```

`catalog` profile 保留 release 优化但用 `panic = unwind`，配合逐文件的
`catch_unwind`，一张坏图只会变成目录里的一条 `metadata_error`，不会中断整轮扫描。

```plaintext
$ war3-manager help
Archive management for Warcraft III maps: catalog, dataset and release workflow

Commands:
  build-catalog  扫描地图与压缩包，按 SHA-256 去重，生成静态站点目录
  scan-versions  重读已有数据集的 w3i 版本字段，输出 JSONL
  scan-mods      重扫已有数据集的第三方脚本修改，输出 JSONL
  rescan         重读全部元数据并补齐缺失封面，输出 JSONL
```

### 建立数据集

```bash
cargo run --profile catalog -p war3-manager-cli -- build-catalog \
  /path/to/downloads \
  --out-dir /path/to/war3-maps-dataset \
  --hf-repo magicwenli/war3-maps
```

输入文件不会被修改。递归扫描 `.w3x` / `.w3m` / `.w3n` 以及 `.zip` / `.rar` / `.7z` /
tar 里的成员（依赖系统 `bsdtar`，`--no-archives` 可关闭），按内容哈希去重后写出：

```text
war3-maps-dataset/
  objects/55/55d87b….w3x    # 每个 SHA-256 一份
  covers/55/55d87b….webp    # 封面缩略图
  catalog/maps.json         # 目录（站点读这个）
  catalog/maps.jsonl        # 每行一条，便于下游处理
  catalog/failures.json     # 读不出来的输入，留待复查
```

重复来源保留在 `source_paths` 里。地图名中的颜色代码和未替换的 `TRIGSTR_*` 会被清理，
原始文件名保留。战役包目前记为 `content_type: "campaign"`、
`parse_status: "metadata_unavailable"`，仅按文件名可搜。

### 上游修好一类保护图之后

不必重建目录，重扫回填即可（栏目归属和来源路径都会保留）：

```bash
just rescan /path/to/war3-maps-dataset
```

展开就是这三步：

```bash
cargo run --profile catalog -p war3-manager-cli -- rescan <dataset> -o rescan.jsonl
python3 deploy/apply_rescan.py <dataset> rescan.jsonl
python3 deploy/export_covers.py <dataset>
```

## deploy 脚本

- `merge_dataset.py` — 将新解析批次按 SHA-256 增量合并
- `apply_rescan.py` — 回填 `rescan` 重读出的元数据（名称、作者、玩家数、w3i 字段、修改检测）
- `apply_mods.py` — 回填 `scan-mods` 的检测结果，见 [MOD_DETECTION.md](docs/MOD_DETECTION.md)
- `apply_versions.py` — 回填 `scan-versions` 的版本字段
- `export_covers.py` — 把封面编码成 `covers/<前缀>/<sha256>.webp`，写入 `cover_path` / `cover_url`
- `hf/upload_dataset.py` — 校验并上传内容寻址数据集
- `verify_final.py` — 发布后核对栏目、失败数、封面与下载链接

发布顺序：`merge_dataset.py` → `export_covers.py` → `upload_dataset.py` → `verify_final.py`。
封面必须在上传前导出，否则目录里的 `cover_url` 会指向数据集中不存在的文件。

## 依赖说明

全部走 crates.io，没有 git 依赖也没有 `[patch]`：

```text
war3-manager-cli → war3parser 0.5.1 → war3-mpq 0.9
```

MPQ 的修复以 [`war3-mpq`](https://crates.io/crates/war3-mpq) 独立发布，war3parser
以重命名依赖引入。早先用 `[patch.crates-io]` 替换 `mpq` 的做法只在最终工作区根生效，
下游拿不到，所以换成了独立 crate。

## License

MIT
