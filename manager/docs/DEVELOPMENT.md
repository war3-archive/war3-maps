# 开发与职责边界

`war3-manager` 只维护地图归档、目录、数据集和发布工作流。通用格式能力应在对应项目中
完成，再由这里调用公开接口。

| 内容 | 项目 |
|---|---|
| MPQ 读取、解密、解压和安全校验 | [war3-mpq](https://github.com/war3-archive/mpq-rust) |
| 地图格式解析、通用模型与 WASM API | [war3parser](https://github.com/wesleyel/war3parser) |
| Catalog schema、栏目、数据集和发布流程 | `war3-manager` |

## 修改归属

发现问题时先判断它属于哪一层：

- 任意 MPQ 使用者都会遇到的问题，提交到 `war3-mpq`
- 任意 Warcraft III 地图工具都能复用的能力，提交到 `war3parser`
- 只与当前数据集、目录字段或发布流程有关的逻辑，保留在 `war3-manager`

如果一个需求跨越多层，应拆成独立提交或 PR。例如先在 `war3parser` 增加通用解析 API，
再在 `war3-manager` 中增加 catalog 回填逻辑。不要在同一个提交里同时修改通用解析行为
和数据集策略。

## 上游贡献

通用修改应从目标项目的最新 `main` 创建分支，并在目标项目中完成测试。不要将
`war3-manager` 的提交历史或运维文件带入上游 PR。

建议为目标项目使用独立 clone 或 worktree：

```bash
git clone https://github.com/wesleyel/war3parser.git
cd war3parser
git switch -c feat/<topic>
```

上游合并并发布后，再回到 `war3-manager` 更新调用代码和 lockfile。

## 本仓库验证

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo build --profile catalog -p war3-manager-cli --locked
```

涉及 Python 运维脚本时，应先用临时数据集或 `--dry-run` 验证，避免直接修改正式发布目录。
