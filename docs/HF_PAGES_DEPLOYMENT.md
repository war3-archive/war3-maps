# Hugging Face Dataset + GitHub Pages 部署

## 数据流

清洗工具产出的目录是唯一上传输入：

```text
dataset/
├── objects/ab/<sha256>.w3x
└── catalog/
    ├── maps.json
    └── maps.jsonl
```

`catalog/maps.json` 是权威索引。GitHub Pages 构建时复制一份精简索引到站内，搜索完全在浏览器执行；地图文件不进入 Git/GitHub Pages。每条记录的 `dataset_path` 会转换成以下单文件 URL：

```text
https://huggingface.co/datasets/<owner>/<dataset>/resolve/main/<dataset_path>?download=true
```

每条记录还包含可搜索的基础信息：作者、简介、推荐人数、实际玩家人数
(`player_count`)、最大人数 (`max_players`)、w3i 版本 (`format_version`)、
地表 (`tileset`)、可玩区域尺寸，以及封面缩略图。封面从地图内的
`war3mapPreview.tga/blp`（优先）或 `war3mapMap.tga/blp`（回退）提取，压缩为
最长边 128px 的 JPEG，并以 `data:image/jpeg;base64,...` 直接写入记录的
`cover_data` 字段，不额外产生文件；`cover_source` 记录来源是 preview 还是
map。没有内嵌预览图的地图该字段为空，网站显示占位背景。

## 1. 上传公开 Hugging Face Dataset

先把 `deploy/hf/DATASET_README.template.md` 按实际来源、许可证、联系方式补全并放到数据集根目录的 `README.md`。确认你有权公开分发地图，再执行：

```bash
python3 -m venv .venv-hf
.venv-hf/bin/pip install -r deploy/hf/requirements.txt
HF_TOKEN=hf_... .venv-hf/bin/python deploy/hf/upload_dataset.py \
  /path/to/dataset \
  --repo-id <owner>/<dataset>
```

脚本默认创建**公开** dataset，上传前检查 SHA-256 唯一性、对象是否存在、大小及实际内容哈希。90GB 首次上传可重复运行同一命令；Hugging Face 的 Xet 上传会跳过已经提交的内容。先用 `--dry-run` 验证，或先上传一个小样本确认数据卡和下载行为。重试时若已经独立校验过本地目录，可用 `--skip-hash-check` 省去再次读取 90GB。

不要把 token 写进仓库。若使用 GitHub Actions，同名 secret 应为 `HF_TOKEN`；90GB 首次导入更适合在本机运行，避免 Actions 时限。

某个下载栏目完成后，可以先解析为独立批次，再增量合并到发布目录。合并按 SHA-256 去重，只复制新增对象，并保留栏目来源：

```bash
python3 deploy/merge_dataset.py /path/to/dataset /path/to/batch \
  --collection '休闲+小游戏' --target-collection '战役包'
```

## 2. 构建地图目录站

已有 catalog 时可在本机组装完整站点：

```bash
python3 deploy/build_pages.py \
  --catalog /path/to/dataset/catalog/maps.json \
  --hf-repo <owner>/<dataset> \
  --out site-dist
```

输出站点根页面是地图搜索，`/parser/` 保留 WASM 本地解析器。构建器同时接受 JSONL；没有本地 catalog 时也可使用 `--catalog-url`。

## 3. 部署 GitHub Pages

1. 创建 public GitHub repo 并推送代码。
2. 在仓库 **Settings → Pages → Build and deployment** 选择 **GitHub Actions**。
3. 在 **Settings → Secrets and variables → Actions → Variables** 添加：
   - `HF_DATASET_REPO=<owner>/<dataset>`
4. 运行 `Deploy catalog and parser to GitHub Pages` workflow。

workflow 会从 Dataset 的 `catalog/maps.json` 读取索引、构建站点并发布。Dataset 更新后手动重跑 workflow 即可刷新 Pages；不需要把 90GB 文件再传给 GitHub。

## 版本与不可变性

网站默认使用 `main`，方便目录即时更新。地图对象以 SHA-256 命名，应视为不可变；更新只新增对象或修正 catalog。若需要完全可复现的历史站点，把 Pages 配置中的 revision 改成 Hugging Face commit SHA。
