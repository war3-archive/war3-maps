# 第三方脚本修改检测

目录里的 `modification` 字段记录的是：这张地图的脚本里检出了已知第三方工具注入的代码。
检测只看单个文件，不需要拿到原版地图做对比。

## 为什么单文件就够

注入工具会把自己的横幅字符串写进 `war3map.j`，而横幅同时被脚本自身引用（菜单标题、
暗语口令、作者身份判断都是从横幅里按字节切片取的），删掉横幅功能就坏了。所以横幅是
一个删不掉的特征，扫描 `WuHansen`、QQ 号 `21764538` 这类字面量即可命中。

## 目前收录的工具

### HKE 作弊脚本（火龙）

由 `HkeW3mModifier` 注入，官方说明见 <https://www.wuhansen.com/warmap/>。

**触发方式**（与注入的 JASS 实际注册的事件一致）：

| 方式 | 说明 |
|---|---|
| 方向键 ↑↑←↓ | 开启作弊。四个方向键各注册一个 `TriggerRegisterPlayerKeyEventBJ`，驱动一个每玩家独立的状态机，按错归零、不限时间。第一个开启的玩家成为 CheatMaster，其他玩家无法再开 |
| 方向键（开启后） | 操作弹出菜单，并直接生效：→ 加钱木，← 清 CD、建筑瞬间完成，↓ 血魔满，↑ 组合键加属性 |
| `-` 开头的聊天命令 | 一个非精确匹配的聊天事件接管所有 `-` 命令，`-h` 查键盘作弊、`-c` 查命令列表、`-u` 查单位类命令 |
| 暗语 `iamWuHansen` | 脚本内无明文，写成 `"iam"+SubStringBJ(横幅,139,146)` |
| Esc | 部分版本（CheatEngine1.25）注册按键释放事件与 `EVENT_PLAYER_END_CINEMATIC`，用于清 CD 和切换背包 |

已见到的横幅版本：`Hke1.25B`、`hke1.25B`、`orz1.25B`、`CheatEngine1.25`。注入代码的
变量前缀每张图不同（`hke_`、`dianso_`、`COWUHVCQDW_`），但横幅与变量骨架一致。

## 运行

检测已经内置在 `build-catalog` 里，新建目录时自动写入。对已有数据集重扫：

```bash
cargo run --profile catalog -p war3-manager-cli -- scan-mods /path/to/dataset -o mods.jsonl
uv run --project deploy war3-deploy apply-mods /path/to/dataset mods.jsonl
```

`scan-mods` 默认只输出命中或读取失败的对象，`--all` 输出全部。`apply-mods` 按
SHA-256 回填：扫描里没有的对象保持原样，扫描里出现但没有 `modification` 的会被清除，
所以撤下一条特征后站点上的标记也会跟着消失。

## 限制

**没有标记不等于干净。** 受保护的地图读不到脚本，这类对象在扫描里记为不可读，不是
"未检出"。

**检出不等于不可玩。** 注入的是一个需要主动开启的菜单，地图本身的玩法完好。相当一部分
老图现在只剩被改过的版本在流通，标记比下架更合适。

**特征会漂。** 工具有多个版本，新版本可能换横幅。新增特征后重跑 `scan-mods` 即可，
不需要重建目录。
