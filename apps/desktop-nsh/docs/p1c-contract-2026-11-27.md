# P1c 桌面客户端 · 隔离实例契约报告（单源浏览 / 短剧 / 竖屏连播）

日期：2026-11-27（Asia/Shanghai）；起点 HEAD `2d8e9460`，实际工作区 HEAD `bd41ed4a`（外部提交 `8d191db3` CI 隔离、`bd41ed4a` OIDC 动态路由；本轮未改这两处，桌面改动也未提交）
客户端：`apps/desktop-nsh`（moontv-native 0.5.0）
服务端：同一工作区的 Next.js 路由，`NEXT_PUBLIC_STORAGE_TYPE=d1` + better-sqlite3
结论：**69/69 项检查通过，0 失败，0 跳过**。

## 这次验证了什么、没验证什么

|一侧|事实|
|---|---|
|真实的|服务端 `source-search/*` 与 `duanju/*` 路由、中间件鉴权、SQLite、夹具源的 CMS 分类/视频/搜索响应；客户端由本仓 Rust 代码发起请求|
|受控的|本地夹具新增 CMS `class` 列表、父子分类（电影/剧集/短剧 + 短剧·甜宠/逆袭）、分页 2 条/页、短剧 3 条与 `vod_play_from`；一次性 SQLite、端口、数据目录|
|外部的|无新增外部检查；本轮 D05/D06 豆瓣公网检查沿用上一轮，网络不可达时会记为 `skip (external)`|
|未验证|真实部署实例、真实第三方采集源（含脚本源/special 源）、真实短剧源与竖屏视频比例、Linux、多屏/混合 DPI、真实断网降级、自动连播的完整人工时序|

## 怎么复现

```powershell
node apps/desktop-nsh/tests/contract/isolated-run.mjs
```

最终运行的机器可读证据：`docs/p1c-contract-2026-11-27.json`（生成于 2026-11-27T02:47:17Z，实例 `127.0.0.1:58820`）。

## 本轮新增检查

| id | 检查 | 结果 | 实测细节 |
|---|---|---|---|
| W01 | 单源源列表需要登录，只返回账号可用源 | pass | 匿名 401；登录后 `fixture-a/b/c` 3 个 |
| W02 | 片源分类返回「类型 → 分类」父子关系 | pass | 5 个 class；短剧 `8`，子类 `81.pid=8` |
| W03 | 单源分类第 1 页：2 条、pageCount=2、每条带可播集表 | pass | `episodes` 全为 `.m3u8`，source 保持 `fixture-a` |
| W04 | 单源第 2 页不重复第 1 页，末页 1 条 | pass | `page=2`，1 条，id 不与第 1 页重叠 |
| W05 | 源内搜索按关键词返回该源自己的结果 | pass | `回声` 2 条，标题包含关键词，`source_name` 保留 |
| W06 | 不可用源 404，缺分类/缺关键词 400 | pass | `source=404`（含 nope）、`category=400`、`keyword=400` |
| U01 | 短剧源只包含声明了短剧分类的采集源 | pass | 只返回 `fixture-a`（`typeId=8`，`typeName=短剧`）；夹具 C 不含短剧被排除 |
| U02 | 短剧分类保留一级短剧与子分类 | pass | 3 个：`8`、`81(pid=8)`、`82(pid=8)` |
| U03 | 短剧第 1 页：2 条、pageCount=2、每条带 m3u8 集表 | pass | 3 条短剧夹具按 2 条/页 |
| U04 | 短剧第 2 页不重复第 1 页 | pass | 1 条，id 不重叠 |
| U05 | 未知短剧源是 404，不返回空成功 | pass | `404` + `未找到短剧采集源: nope` |
| U06 | 热播短剧返回带可播集表的条目 | pass | 2 条，全部 `.m3u8`；无条目时是空数组而非错误 |
| R01 | 客户端真实代码走完登录/搜索/SSE/建议/历史/单源/短剧/详情/收藏/记录/日历/刷新/退出 | pass | `cargo test --test live_contract` exit=0 |

R01 新增的客户端断言：源列表 3 个；分类保留父子关系；单源第 1/2 页与分页不重复；源内搜索命中 2 条；不可用源映射为可读错误；短剧源只有 1 个且 `type_id=8`；短剧分类/分页/推荐；竖屏记录 `is_duanju=true` 写入并读回。自动连播、竖屏布局和详情入口由 UI 截图与人工操作验证，不在 R01 内。

## 这一轮发现的真实差异

1. **短剧四个路由在 middleware 下同样需要登录**：路由本身不读 cookie，但 `/api/duanju/*` 没有在 middleware 白名单里，未带凭据会 401。客户端必须像其他 API 一样带 Bearer；不能因为路由代码没有 `authInfo` 就匿名调用。
2. **短剧分类是 `{code,data}`，单源分类是 `{categories}`**：客户端分别解析 `data` 与 `categories`，不共用一个字段名。
3. **单源搜索结果集只有 `source/source_name`、没有 `type_name`**：`episodes` 需要服务端同时拿到 `vod_play_from` 才会填充；缺 `vod_play_from` 的源在列表有行、详情可能报“没有可用剧集”。夹具已补上该字段以贴近真实 CMS。
4. **短剧推荐是全局内存缓存 + 首个源**：`duanju/recommends` 依次尝试短剧一级分类与子分类，过滤集数 0 与 `.m3u8`；它可能为空，客户端把空推荐与失败分开显示。
5. **页码字段是 `pageCount`**（服务端统一 reshape 后的 camelCase），客户端的“加载更多”以 `page < pageCount` 为准；末页/重复页不再请求，去重后新增为 0 即停止。
6. **夹具 URL 双 scheme 修正**：夹具模板 `http://127.0.0.1:PORT` 过去被替换成 `http://host`，产生 `http://http://...`；契约的列表检查不敏感，但真正播放会失败。本轮把替换改为 host:port，竖屏截图才能真实播放受控媒体。这是夹具修复，不是服务端协议变化。

## 仍未验证（不要当成通过）

- 真实实例、真实采集源、脚本源/special 源分类与分页；真实短剧源的竖屏视频比例、自动连播完整时序。
- 单源 `special=1` 版本：客户端尚未实现 special 源入口。
- 播放后的浏览位置/滚动恢复：当前依赖内存中的路由与状态保留，没有做持久化快照。
- 真实断网/源站超时的 UI 恢复、Linux、多屏/DPI、触摸/手势。

## 相关文件

- 截图：`apps/desktop-nsh/docs/screenshots/`（23–31，全部为隔离实例，没有用演示冒充）
- 夹具与驱动：`apps/desktop-nsh/tests/contract/`
- 客户端：`src/source_browse.rs`、`src/state.rs`、`src/api.rs`、`src/ui/pages.rs`、`src/ui/player.rs`
- 机器可读报告：`apps/desktop-nsh/docs/p1c-contract-2026-11-27.json`
