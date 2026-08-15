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
| `classify-tags` | 通过本地小模型或远程 API（如 OpenRouter）生成可续跑的玩法、系列和题材标签候选 |
| `export-covers` | 导出 WebP 封面并写入 `cover_path`、`cover_url` |
| `upload` | 校验并上传到 Hugging Face |
| `verify` | 发布后检查栏目、失败记录、封面与下载链接 |

每个长流程都会在 stderr 打印进度（终端里就地刷新，重定向时每步一行），JSON 报告写
在 stdout，所以 `war3-deploy verify ... | jq` 不会被进度污染。

## classify-tags 模型来源

默认连本机 llama.cpp / MLX 服务（`http://127.0.0.1:8080`）。要改用 OpenRouter 等
远程 provider，把 `deploy/.env.example` 复制为 `deploy/.env` 并填入：

```bash
cp deploy/.env.example deploy/.env
# 编辑 .env：填 OPENROUTER_API_KEY，确认 endpoint / model
```

程序会显式读取项目目录的 `deploy/.env`；等价的环境变量：`WAR3_TAG_LLM_ENDPOINT`、`WAR3_TAG_LLM_MODEL`、
`OPENROUTER_API_KEY`（也可用 `--endpoint` / `--model` / `--api-key` 覆盖）。
没有 API key 时不会发送 `Authorization` 头，本地服务不受影响。

对 OpenRouter 的默认请求会发送 `reasoning.enabled=false`，避免推理 token 挤占批量 JSON 输出；模型仍
发生截断时，程序会自动将这一批对半重试。词表已有较完整的基础项，模型也可默认提出
受限的新标签（仅 `玩法:`、`系列:`、`题材:` 三个命名空间，每图最多三个）。使用
`--no-new-tags` 可切回严格的固定词表。

远程模型可用 `--workers 8` 并发请求以缩短全量生成时间；检查点仍由单一主线程写入，
中断后可直接续跑。若 provider 报并发或速率限制，降低该数值。

首次上传数据集前，按实际来源、许可证和联系方式补全 `hf/DATASET_README.template.md`，
并将其保存为数据集根目录的 `README.md`。完整发布流程见
[../docs/OPERATIONS.md](../docs/OPERATIONS.md)。
