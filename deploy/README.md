# war3-deploy

数据集维护与发布工具，由 [uv](https://docs.astral.sh/uv/) 管理。所有命令都作用于
同一个内容寻址数据集（`objects/`、`covers/`、`catalog/`），共享的目录读写、原子写入
和进度输出集中在 `war3_deploy.catalog` 与 `war3_deploy.progress`。

```bash
uv run --project deploy war3-deploy --help
```

| 子命令 | 作用 |
|--------|------|
| `merge` | 按 SHA-256 增量合并新批次 |
| `apply-rescan` | 回填 `war3-manager rescan` 结果 |
| `apply-mods` | 回填 `war3-manager scan-mods` 结果 |
| `apply-versions` | 回填 `war3-manager scan-versions` 的 w3i 版本字段 |
| `classify-tags` | 通过本地小模型生成可续跑的玩法、系列和题材标签候选 |
| `export-covers` | 导出 WebP 封面并写入 `cover_path`、`cover_url` |
| `upload` | 校验并上传到 Hugging Face |
| `verify` | 发布后检查栏目、失败记录、封面与下载链接 |

每个长流程都会在 stderr 打印进度（终端里就地刷新，重定向时每步一行），JSON 报告写
在 stdout，所以 `war3-deploy verify ... | jq` 不会被进度污染。

首次上传数据集前，按实际来源、许可证和联系方式补全 `hf/DATASET_README.template.md`，
并将其保存为数据集根目录的 `README.md`。完整发布流程见
[../docs/OPERATIONS.md](../docs/OPERATIONS.md)。
