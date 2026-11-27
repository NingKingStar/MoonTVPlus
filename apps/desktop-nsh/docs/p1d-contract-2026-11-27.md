# P1d 桌面客户端 · 隔离实例契约报告（多源切换 / 源记忆 / 失败备用源）

日期：2026-11-27（Asia/Shanghai）；起点 HEAD `f8488a27`（工作区未提交）
客户端：`apps/desktop-nsh`（moontv-native 0.6.0）
服务端：同一工作区的 Next.js 路由，`NEXT_PUBLIC_STORAGE_TYPE=d1` + better-sqlite3
结论：**70/70 项检查通过，0 失败，0 跳过**。

## 这次验证了什么、没验证什么

|一侧|事实|
|---|---|
|真实的|服务端 search / source-detail / source-search / duanju 路由、中间件鉴权、SQLite；客户端真实 Rust 代码取回多源结果|
|受控的|本地夹具新增 `fixture-c/2001`：与 `fixture-a/1001` 同名同年的第二片源，2 集 m3u8；`fixture-b` 继续作为失败源；慢源延迟 1.5 s|
|未验证|真实部署实例、真实第三方/脚本源、真实媒体断流/解码错误后的自动换源体验、音画/seek、Linux/多屏/DPI|

## 怎么复现

```powershell
node apps/desktop-nsh/tests/contract/isolated-run.mjs
```

最终证据：`docs/p1d-contract-2026-11-27.json`（生成于 2026-11-27T07:36:09Z，实例 `127.0.0.1:57178`）。

## 本轮检查变化

| id | 检查 | 结果 | 说明 |
|---|---|---|---|
| C08 | 搜索完整条目（含同作品第二片源） | pass | 结果 4 条：`fixture-a/1001` 2 集、`fixture-c/2001` 2 集，同名同年 |
| C08b | 第二片源保留独立 source/id | pass | 客户端可把两条合并为一个 Work 的两个候选，而不是覆盖 |
| C18 | SSE 计数 | pass | 3 源，`fixture-a` 3 条 + `fixture-c` 1 条，complete `totalResults=4` |
| S01 | 增量时序 | pass | 快源先到，慢源/complete 后到；慢源现在真的带 1 条结果 |
| C18b | 失败源模型 | pass | `fixture-b` 仍是空 `source_result`，不触发整批失败 |
| R01 | 客户端真实链路 | pass | 搜索得到 4 条并分成 3 个作品；`回声档案/2026` 有 2 个候选；`next_backup_candidate`/`preferred_source` 断言通过；源内搜索/短剧/详情/收藏/记录/日历/刷新/退出保持通过 |

R01 新增断言：同名同年的第二片源会被分组为同一 Work 的 2 个候选；`next_backup_candidate` 能给出备用源；`preferred_source` 优先记住的源、再次显式源、最后默认排序第一个；`Library` 的 `last_sources` 仅本地序列化并随 `clear_all` 清空。

## 客户端实现与边界

- 详情页把源选择移入「选集」区：显示「当前源：名称 · N 集」，候选源按钮显示名称、集数与「当前」标记；切源保留当前集号，新源集数不足时回落到第 1 集，不套用新源的旧进度。
- `Library.last_sources` 以 `title + 0x1F + year` 为 key 记住最后成功详情/播放的源；进入作品时记忆源 > 显式来源 > 默认排序第一个。只写本机库，不进入收藏/记录/设置同步载荷。
- 播放错误时先按 Work 候选顺序尝试未试过的源，每个源最多一次；有集表直接切，缺集表时取 `source-detail` 再切；头部 notice + toast 明确显示「已切换到备用源：X」。全部失败恢复第一次错误并显示重试按钮；手动换源/重试会重置尝试列表。
- 自动换源已用隔离实例的坏媒体（`FIXTURE_BAD_MIRROR=1`）实际触发：日志从 `fixture-c` 的 `play_fallback` 切到 `fixture-a` 并重新 `play_start`；但截图窗口重建时提示条超时，没有留下可见提示条画面。真实媒体断流的手感仍需人工验收。

## 截图

隔离实例、窗口级截图，未抢占前台：

- `docs/screenshots/33-search-multi-source.png`：`回声档案/2026` 标「2 个片源」，同名的 2020 年作品仍是单源。
- `docs/screenshots/34-detail-source-tabs.png`：在详情页实际点击夹具 C 后，选集区显示「当前源：夹具 C（慢）· 共 2 集」与两个候选的当前标记，当前集仍为第 1 集。
- `docs/screenshots/35-detail-source-default.png`：未命中记忆时的夹具 A 默认状态。

自动备用源的失败提示条没有留在截图里：坏媒体触发了回退并写日志，但重建播放窗口后提示条已超时；不拿失败态冒充通过。

## 仍未验证

- 真实实例、真实第三方/脚本源、真实媒体断流/解码错误后的自动换源手感（需要用户在 UI 中制造失败并观察提示）。
- 播后滚动位置持久恢复、字幕/倍速/下载/管理后台（本轮不做）。
- Linux、多屏/混合 DPI、真实音画与 seek 精度。

## 相关文件

- `src/model.rs`（`last_sources`、`preferred_source`、`next_backup_candidate`）
- `src/state.rs`（切源保留集号、源记忆、失败备用源、`is_duanju` 之外的现有逻辑）
- `src/ui/pages.rs`（选集区源切换）、`src/ui/player.rs`（备用源提示）
- `tests/contract/fixture-source.mjs`、`tests/contract/isolated-run.mjs`、`tests/live_contract.rs`
