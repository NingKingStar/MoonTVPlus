# MoonTVPlus 原仓 Rust + NSH 全量重构路线图（研究稿）

本轮交付研究、需求和可审阅提案，尚未开始搬代码或开发。用户要求的原仓全量功能均列入重构范围；P0–P3 是交付顺序。原生界面重新设计，默认不复制旧 App 代码，不继承网页 UI。没有推送、发布、触发 CI、部署、运行服务端、安装依赖或执行迁移。

## 从哪里读

- [全量需求矩阵](nsh-rebuild-requirements.md)：130 个稳定编号，每项含动作、源码行、现有平台、NSH 与服务端职责、依赖、验收、优先级和状态；另有 4 项缺失/宣称记录。
- [CSV 总表](nsh-rebuild-requirements.csv)：便于筛选优先级、状态和模块；与主表同源。历史缺口保留在 Markdown，不混入已存在功能。
- [配置逐字段补充表](nsh-configuration-fields.md)：189 项管理合同字段声明、43 个个人设置同步键，继承父需求的实现和验收要求。
- [逐文件入口与证据索引](nsh-baseline-inventory.md)及 [机器可读哈希基线](nsh-baseline-inventory.json)：全部页面/API 方法、动作声明、文件归属与 SHA-256。索引归属不是逐行安全审计或运行证明。
- [CI 未应用补丁](nsh-ci-isolation.proposed.patch)、[决策提案审阅副本](nsh-decisions-proposed.md)。

## 精确基线与工作区边界

|项目|事实|
|---|---|
|目录|C:/project/MoonTVPlus|
|origin fetch/push|https://github.com/Happier-X/MoonTVPlus.git|
|分支 / HEAD|main / 5b132342e7d36282e67edd73d188468d8c446ac6|
|盘点日期|2026-11-24，Asia/Shanghai|
|首次本地状态|git status --short 为空；最初未发现适用 AGENTS|
|后出现的其他改动|package.json 增加 notes 脚本，新增 AGENTS.md、.github/workflows/verify-notes.yml，忽略目录下出现 .agents 规则与脚本；不是本研究写入，保留不覆盖|
|适用规约|后来出现的根 AGENTS 与 .agents/notes/AGENTS.md 已读取；按 proposed 记架构/发布取舍并校验，未初始化或替换规约|
|只读原生参考|C:/project/MoonTVPlus-NSH；用户亲测演示可用；独立于原仓 HEAD，以索引中的文件哈希标记此次读取版本|
|历史研究|C:/project/MoonTVPlus-PC/docs/nsh-native-app-research.md 仅历史背景，不能覆盖当前 HEAD|

本轮自有变更在 docs/ 和两篇 .agents/notes/proposed/ 下。HEAD 的 .gitignore 忽略 .agents，笔记因此另有 docs 审阅副本；未强制加入 Git、未改忽略规则。新出现的 notes workflow 依赖被忽略脚本，直接提交该 workflow 可能缺脚本，应由该改动负责人补齐追踪方案。本轮不代其修改。

## 全量覆盖与尚未完成的真实能力

从精确 Git 对象索引 759 个跟踪文件中的 687 个文本配置/代码/文档：全部 32 个页面、135 个 API route、120 个 src/lib 文件、214 个 apps 文件、11 个 HEAD workflow。当前 HEAD 的产品 Markdown 共 6 份：根 README、SOURCE_SCRIPT、VERCEL_DEPLOYMENT 及三份 apps README；全部纳入。还盘点 components/hooks/types、Socket/worker、脚本/迁移、package scripts/部署配置及 public 下推送/离线缓存 worker、Bangumi 代理和字幕 worker。431 处矩阵证据的文件存在和行范围已核对；没有向真实实例发出请求。

|模块|需求项|覆盖内容|
|---|---:|---|
|基础 BAS / 账号 AUTH|2 / 11|连接实例、原生导航；密码/注册/Turnstile/OIDC/二维码/Telegram、刷新、设备和权限|
|发现 DIS / 搜索 SRC / 短剧 DJS|9 / 8 / 2|首页、豆瓣榜单分类筛选、番组计划、TMDB、脚本推荐、单源浏览；聚合/SSE/筛选/历史；短剧独立浏览与竖屏连播|
|详情 DET / 播放 PLY|5 / 16|元数据/评论/推荐/剧集；原生媒体、多源、进度、轨道、全屏/小窗、跳过、预缓存、截图/缩略图、外部播放器、超分|
|字幕 SUB / 弹幕 DM / 去广告 AD / 下载 DL|3 / 4 / 2 / 5|文本/ASS/PGS、匹配与渲染/过滤/热力图、内置/自定义过滤、本机下载与离线、服务器下载|
|直播 LIVE / 电视设备 TV|5 / 7|M3U/EPG/代理/历史、虎牙/B站/抖音；大屏焦点、虚拟遥控、TVBOX/OrionTV、DLNA、系统媒体和 Android 更新|
|片库 LIB / 偏好 CFG / 通知 NTF / Telegram TG|4 / 5 / 4 / 1|收藏/历史同步、清理、个人设置/云备份、缓存、收件箱/更新提醒/邮件/Web Push/绑定|
|AI / 观影室 ROOM|2 / 6|流式问答与设置、AI 短评；房间/密码/房主/同步/聊天/语音/屏幕共享与连接基础|
|管理 ADM / 运维 OPS|19 / 10|用户/组/设备/配置订阅/片源脚本/主题/邮件/TG/认证/重置；存储、备份迁移、任务、Linux/多平台、CI/包/安全边界|
|合计|130|另有 GAP-01..04 四项历史缺失/宣称记录；磁力已作为 SRC-08 条件缺口纳入主表|

|维度|数量|解释|
|---|---:|---|
|P0 / P1 / P2 / P3|23 / 41 / 58 / 8|按每项主要启动阶段计数；跨阶段项目要完成后续子门，不能只完成 P0 子集就关闭父项|
|M 已原生本地模拟|8|导航、核心播放/中继/选集/连播、局部全屏、本机片库、安全中继样本；演示媒体与受控样本，不是真实服务通过|
|R 真实待联调|10|实例/登录/搜索/详情/收藏/记录/刷新已有原生代码；这 10 项都已在本仓服务端的隔离实例上跑通契约，真实部署实例、权限组合与生产数据仍未验|
|U 未做|112|包括没有原生实现或仅写了方案的功能；写文档不算完成、不算验收|
|真实接口验收通过（生产实例）|0|本轮没有任何生产实例访问；不以 mock 或隔离实例通过代替生产兼容验收|
|隔离实例契约检查|P0 39/39；P1 57/57|本仓服务端代码 + 一次性 SQLite 库 + 受控片源，仅 127.0.0.1；P0 见 [P0 契约报告](../apps/desktop-nsh/docs/p0-contract-2026-11-24.md)，P1 见 [P1 契约报告](../apps/desktop-nsh/docs/p1-contract-2026-11-24.md)。P1 的 D05/D06 为外部豆瓣公网数据，单独标注|

## 2026-11-24 P0 实施进展（与上面的研究稿分开读）

研究稿的“尚未开始搬代码”描述的是文档交付时点。此后用户授权 P0 实施，当前工作区已有：

|项目|状态|证据|
|---|---|---|
|`apps/desktop-nsh` 原生工程|moontv-native 0.3.0，单 crate、独立 `[workspace]`、NSH 固定 revision、逐文件来源哈希|`apps/desktop-nsh/provenance.json`、`Cargo.lock`|
|构建门禁|fmt / clippy `-D warnings` / 13 单测 / release 构建全绿|本地实测，命令见 `apps/desktop-nsh/dist/staging/BUILD.txt`|
|真实服务端契约|登录、刷新与 4h/60d 规则、401/403/429、搜索与 SSE、source-detail 与 UA 代理、收藏与观看记录读写合并、退出撤销|`apps/desktop-nsh/docs/p0-contract-2026-11-24.md`（39 项）|
|便携试用包|`moontv-native-0.3.0-windows-x64-portable.zip` + SHA256，未签名、无安装器|`apps/desktop-nsh/dist/`（不入库）|
|CI 隔离|本地已落 `cloudflare-deploy.yml` 路径忽略/环境保护/停止打印 wrangler，新增 desktop 专用验证工作流；**未提交、未推送、未触发**|`.github/workflows/`|
|未验证|真实部署实例、Linux 桌面、多屏/DPI、硬解、Turnstile 实例、干净目标机、签名与安装更新|见契约报告“未验证”一节|

M 是“有该项局部本机演示证据”，不表示该项全部动作已完成。例如 PLY-07 的全屏有实现，最终包 F/F11/按钮/Esc 键矩阵仍待复验，PiP 未做；OPS-10 仅中继样本已有，完整权限/脚本安全验收未做。

## 2026-11-24 P1 实施进展（增量，未提交）

P0 之后只做了 P1 的第一步，范围是搜索增量/建议历史/豆瓣分类/每日放送；部署、CI、Linux 联调没有动。

|项目|状态|证据|
|---|---|---|
|版本与包|moontv-native 0.4.0；便携包 `moontv-native-0.4.0-windows-x64-portable.zip` + SHA256（包内 exe 与 `cargo build --release` 产物哈希一致）|`apps/desktop-nsh/dist/`（不入库）|
|SRC-02 SSE 增量|逐源增量展示、中文跨块/多行事件解析、取消不覆盖新查询、单源超时保留已到结果、断流降级到 `/api/search` 并去重合并|`src/search_stream.rs`、`src/state.rs`、`tests/stream_contract.rs`、隔离契约 S01|
|SRC-03 建议/历史|300 ms 防抖与过期建议丢弃；历史本机缓存 + 服务端增删清空，清空二次确认|`src/state.rs`、`src/api.rs`、隔离契约 G01–G03/H01–H07|
|DIS-02/DIS-03 豆瓣分类|电影热门/最新/高分/冷门、电影/剧集/综艺二级分类；原生多级筛选（类型/地区/年代/平台/排序）、筛选记忆、分页去重与失败重试；全部走实例 `douban/categories` 与 `douban/recommends`|`src/discover.rs`、`src/ui/pages.rs`、隔离契约 D01–D06、截图 15/16/21|
|DIS-04 每日放送|周一至周日切换、当天预选、中文名缺失回退原名、0 分不显示评分；数据走实例 `bangumi/calendar`|`src/discover.rs`、隔离契约 B01/R01、截图 20|
|界面回归|深色影院风与左侧导航保留，新增「发现」；1123×593 pt 窄窗下连接/设置改为纵向、搜索栏可收缩，无横向溢出；Ctrl+K、全屏/播放快捷键未改|截图 11–22、`docs/screenshots/README.md`|
|构建门禁|fmt / clippy `-D warnings` / 33 单测 + 2 集成测试（live_contract 默认可跳过）/ release 构建全绿|本地实测；见 `dist/staging/BUILD.txt`|
|隔离实例契约|57/57 通过（含 S01 增量、G/H 建议历史、B01 日历、D01–D06 豆瓣参数与外网数据、R01 客户端真实链路）|`apps/desktop-nsh/docs/p1-contract-2026-11-24.md`、`p1-contract-2026-11-24.json`|
|未验证|真实部署实例、真实第三方片源、豆瓣/Bangumi 上游长期可用性、音画/seek/连播人工验收、Linux、多屏/DPI、签名与更新|见 P1 契约报告“仍未验证”|

豆瓣 D05/D06 是外部公网数据，不把它当作夹具或生产实例证据；外网不可达时契约会记为 `skip (external)`。

## README、现有实现与缺失功能的界线

|候选或宣称|实际核对|重构处理|
|---|---|---|
|推荐/豆瓣分类榜单|有实际首页、hot/latest/highscore/rare 和电影/剧集/综艺/动漫筛选，多级分类含地区/年/平台/排序；Bangumi 日榜/排期、TMDB 上映/趋势、脚本推荐另有入口|DIS 全量保留；不把静态推荐称个性化算法|
|实时搜索|/api/search/ws 实际是 SSE，普通 JSON 搜索另在 /search；有增量结果/源错误/完成状态|SRC-01/02 分别实现，不按路由名字误接 WebSocket|
|原生演示与真实 App|参考工程虚构作品/受控媒体可播、可收藏续播；已写部分登录/search/detail/favorites 适配；从未证明真实服务器兼容|保留演示成果；真实闭环单独验收。现有 Compose Desktop 也是另一实现，其内嵌播放有占位并可打开浏览器|
|磁力/ACG|KMP MoonApi/AcgModels/SearchRepository 残留四源合同；HEAD 无对应网页或 /api/acg 服务|SRC-08 条件恢复项，不承诺已有磁力搜索或 BT 播放|
|音乐/书籍/漫画/私人影库/追番订阅/网盘/求片|config.ts:518–537 明确下线漫画 Suwayomi、网盘 NetDisk/Pansou 和求片配置，:816/820 清理其他旧配置，015 迁移删除音乐书籍；漫画仅旧迁移表|GAP-01..03 记录；没有现存对应页面/API；如用户所指包括历史删除能力，指定历史范围后扩子矩阵，不冒称 HEAD 现有功能|
|离线/PWA|有 PWA 配置，也有真实本机 M3U8/IndexedDB 缓存和服务器离线下载实现；三者不同|分别验收 OPS-09、DL-01..03、DL-04/05；服务器下载是管理员功能且需要持久盘|
|字幕/去广告/超分|网页 JASSUB ASS/SSA、libbitsub PGS、HLS 过滤、自定义 JS、Anime4K 有代码；Android 基础 ASS 不等于完整效果|重写原生媒体桥；复杂字幕/图像后处理显式依赖；自定义 JS 不在 Rust 进程裸执行|
|观影室云端限制|Node Socket.IO 和 Cloudflare Durable Object/hub 均有实现；CF 不能一概说没有长连接|ROOM-06 按具体部署验证；Vercel/其他平台需相应连接服务，不能泛化支持|
|网红直播|当前实现是虎牙、Bilibili、抖音的 HLS/FLV 解析|LIVE-05 只承诺现有平台，依赖上游接口实际可用|
|Netlify 等部署|README 宣称支持 Netlify，但当前无充分发布配置证明；Vercel/CF/EdgeOne/Docker 等有配置/脚本|OPS-06/GAP-04 记录平台差异，未实测不标正式可用|
|设置同步|有白名单/version=1/64KiB/MD5 和 409 分支|CFG-03 必须实测并发冲突，不宣称已有原子 CAS；不把本机路径或密钥上传|

所有当前实际功能默认在范围内；历史删除项和设备受限项保持开放台账，不能静默删除。外部播放器按 OS 适用性实现对应能力，不要求 Windows 能运行 IINA 等其他系统软件。

## 分阶段交付与退出标准

复杂度是规模和技术不确定性：S=局部交互/映射，M=一个完整流程，L=多状态/合同/持久化，XL=跨平台媒体/协议/恢复等多系统配合。不换算未经验证的人天或日期。矩阵依赖表示功能所需能力；同一父项可有分阶段子门，不能直接把模块引用当任务 DAG。

|阶段|具体交付|退出标准和先决条件|
|---|---|---|
|P0-A 同仓与隔离基础|拟定 apps/desktop-nsh 独立工程边界、NSH pin/构建基线、CI 隔离落地任务；明确一个隔离 Linux 副本及备份回滚规程|先审阅本轮方案；生产 CI 门禁另行配置，不因提交默认授权部署；真实环境仅用明确隔离数据|
|P0-B 真实身份和服务合同|连接实例、密码登录、凭据持久化/刷新/退出、401/403/429、账号组源权限；启用 Turnstile 的目标实例补安全浏览器 handoff|服务端基线和实际存储类型固定；测试用户有正确权限。禁止通过关闭站点校验来宣称兼容；旧模式能力单列|
|P0-C 找片至续播闭环|真实多源 JSON 搜索→详情/实时集表→选源选集→真实 HLS/MP4 鉴权播放→收藏与记录→重启/换端恢复，基本连播、错误重试|至少两个源，真实失败/空集/超时样本；有声、seek 实际位置、1 基集号和秒单位正确。旧网页/Android 的同账号数据无回归|
|P0-D 可交付基线|fmt/clippy/test/合同/媒体分层报告、干净 Windows 目标机可复现构建/试用包；版本/SHA/能力说明；Linux 版本独立|只对已测 OS/格式宣称支持；ZIP 不冒称签名安装器；备份副本恢复演练通过后才对真实写联调放行|
|P1 日常主体验|豆瓣/番组/TMDB 发现、单源与短剧浏览、SSE/筛选/搜索历史；多源优选、倍速轨道、快捷键、跳过、文本字幕、弹幕、内置去广告；电视直播/EPG；基础本机下载、外部播放器、个人账号设备与常用设置|日常端到端场景可用；外部服务失效有真实错误和降级。DL-01 的首版任务状态/取消须与文件正确性一起做，不能等 P2 才保证安全暂停/失败处理|
|P2 全量功能等值|短剧竖屏完整体验、复杂集表/预载/截图、ASS、自定义过滤、下载管理/离线/服务器任务；全部原生管理、备份迁移/存储/部署运维工具、主题/通知/TG/AI/观影室基础与同步、TVBOX/遥控等|每个 P2 和跨阶段父项按矩阵验收关闭；管理员可在原生完成原有管理动作；Web CSS 与服务配置仍能在原生编辑。条件依赖必须交付或获明确平台限制决策|
|P3 媒体/设备重扩展|PGS、Anime4K、高阶编码/硬解兼容扩展、大屏/投屏、移动系统生命周期映射、语音/屏幕共享等重媒体桥|实际设备/真实对端/网络验证；无设备仍是未测，不用 UI 按钮或 mock 验收。全部受限项有依赖负责人/替代说明，不能无期限挂起即宣称完成|

P0 跨阶段父项边界：OPS-01 首版只固定实际 Linux 后端，其他后端完整合同在 P2；OPS-02 首版备份/副本回滚规程，原生全套导入导出在 P2；OPS-08 首版构建和可试用包，签名安装/更新在 P2；OPS-10 首版凭据/中继边界，脚本隔离和完整安全检查随能力补齐。PLY-02 对脚本懒解析 SRC-04 的依赖仅在脚本源测试时生效：首版 HTTP/HLS 普通源闭环先完成，脚本源不得被标兼容。

基本字幕/弹幕、直播、去广告、本地下载已安排 P1；不会把所有媒体功能机械推到最后。观影室基础连接 ROOM-06、入房/同步/聊天在 P2，语音和屏幕共享才依赖 P3。矩阵的 130 项全部完成后才称当前功能等值；阶段包只能称该阶段范围可用。

## 同仓目录与职责提案

已检查 apps 下仅 android-phone、android-tv、nsh-phone；没有 apps/desktop 或 apps/desktop-nsh。android-phone 内已有 Compose Desktop，不能覆盖。nsh-phone 是 NSH 0.6.1 TSX/Kotlin/Media3 的 Android 宿主，不能当 Rust 桌面 workspace。建议如下目录，尚未创建：

```text
apps/desktop-nsh/
  Cargo.toml                 # 独立 [workspace]，锁文件在本目录
  Cargo.lock
  rust-toolchain.toml
  crates/app/                # 启动、NSH 原生窗口/UI、生命周期
  crates/core/               # 领域类型、播放/搜索/下载状态机
  crates/api/                # HTTP/SSE/Socket、认证、兼容层
  crates/media/              # NSH media 桥、中继、字幕/弹幕接口
  crates/storage/            # 实例/账号隔离、本地记录、缓存、迁移
  crates/platform/           # 凭据库、文件、系统媒体、播放器/投屏
  tests/fixtures/            # 脱敏合同与受控媒体，不能混生产数据
  packaging/                 # 安装包/清单/签名流程，无私钥
```

先按职责划边界，初期可合并过细 crate；有真实跨包需求后再拆，不能为了层数复制 DTO。依赖方向 app → core/api/media/storage/platform；core 不依赖 NSH 或服务端数据库。通过注入接口传递会话、媒体命令、存储和时钟，UI 不直接发零散 HTTP，也不把 OS 句柄写入云设置。

根目录继续承担 Next/Node/Cloudflare 服务。根 tsconfig 既有 apps 排除、.dockerignore 既有 apps 排除，有助隔离但不能替代 CI 路径门禁；pnpm-workspace 的构建依赖政策不等于 Rust workspace。暂不引入根 Cargo workspace；若日后第二个 Rust 宿主需要共享，再用真实依赖决策。

|一侧|长期承担的能力|
|---|---|
|NSH 客户端|全新导航、发现/搜索/详情/播放、片库、设置、下载、设备；原生管理全部交互及任务状态/结果|
|服务端|身份/授权、聚合源与 JS 源执行、元数据代理、同步、通知/第三方凭据、观影室协调、服务器离线任务；保留现有协议并增量新增能力|
|运维|独立 Linux/云服务版本、数据库迁移/快照/恢复、反代与持久盘、定时任务、签名密钥/发布环境；以可审阅 runbook/受控命令或任务交付，不能让桌面直接掌握生产 DB|

网页可作并行过渡入口。退出过渡的条件是：ADM-01..19 全部原生操作与权限验收通过，数据导出/恢复、服务器下载与配置编辑能在目标职责侧完成，常用平台的运维 runbook 可复现；之后是否收敛旧界面仍需单独决定，不自动删网页、Android 或服务端。原生主题用语义化属性，管理端仍需能编辑原有网页 CSS；统计/广告自定义脚本不在客户端盲目执行。

## 原生参考工程的未来迁入策略

当前 MoonTVPlus-NSH 包为 moontv-native 0.2.0、Rust edition 2024；NSH pin 为 2f51d8611677c8a6bbad937b39756e8d0cc45350，含本机 .tools 路径依赖。这只能作为未来可复现依赖整理的起点，不能直接复制到 CI。用户已亲测可用演示保留。

未来逐文件记录来源/许可/新写证明/测试，优先评估独立 Rust 领域逻辑、中继与原生控件经验；真实 API 适配必须按本仓合同重验，再决定选择性迁入还是重写。演示数据只作为明确的 Demo fixture。不得整体搬 target、.tools、日志、profile、token、媒体大文件/构建产物；NSH 改为固定 Git revision 或可审计 vendor，并验证许可证和 Windows 干净构建。此次未迁入任何文件。

已知差异必须先处理：

1. 参考调用 /api/detail；真实播放路径的 /api/source-detail 有实时集表/请求头/特定 UA 的过滤包装等语义，不能只对返回字段相同做假兼容。
2. 参考 User-Agent 为 MoonTVPlus-NSH/0.2 (Windows)，与服务端检查的 moontvplus app/OrionTV 规则不自然相同；应明确客户端类型/能力合同，不靠偶然字符串。
3. 参考未做自动刷新，且把 401/403 都映射为 Unauthorized；要按 4h access/60d refresh/10min 阈值实现并区分权限拒绝。
4. 原生本机历史尚未接服务端 playrecords：原生毫秒与服务秒、一基集号、is_anime/is_duanju/origin、时间戳合并均需真实测试。收藏 source+id 和嵌套 favorite 载荷也要检查完整字段。
5. 原生中继受控 KEY/MAP/Range 样本不等于任意源：headers、重定向、相对 URI、多音轨、live、跨域权限、令牌刷新、断流逐个验收。DLNA 对端不能访问桌面 loopback URL。
6. 仅 Windows 11 build 26300 x64、150% DPI 的局部证据；Linux 桌面/多屏/HTTPS 实例/硬解/完整音画同步未验证。最终包快捷键矩阵也未全部通过。

## Linux 服务独立版本、兼容与灾备

版本钉住、分后端备份、恢复演练、回滚顺序与本机验证边界的可执行清单见 [Linux 隔离实例运行手册](nsh-linux-isolation-runbook.md)；**本轮没有在真实 Linux 上执行过其中任何一步**。

服务端在 Linux 独立部署，不要求把服务塞入桌面安装包。先锁定本轮 SHA 的镜像 digest/配置/存储与文件目录，再在隔离副本建立测试账号和源。Docker full/lite、Node、CF、EdgeOne/Vercel 的持久化、Socket/DO、运行时差异分别声明；lite 不等同 full 的内置观影室，CF 可走 DO，临时盘平台不承诺服务器下载。

建议新增向后兼容的协议版本/capabilities（名称与路径待服务端设计），至少包含认证方式/原生 handoff、存储模式、功能与权限、播放解析/headers、同步版本、流式传输/房间后端、服务器下载能力；站点 Version 不能替代协议版本。旧服务不支持该接口时回退现有 server-config 和原接口，并显示明确缺项，不盲目请求管理路由探测。

|兼容组合|发布前需要证明|
|---|---|
|新桌面 + 现有服务 SHA|现有登录/search/source-detail/favorites/playrecords 核心兼容，不强制用户先升级生产|
|新桌面 + 新增接口服务|hand-off/capabilities/受控过滤按版本协商，旧字段保持；不支持时准确降级|
|现有网页/Android/外部兼容 App + 新服务|原登录、源授权、播放/同步、TVBOX/OrionTV 和 Android 更新通道不回归|
|旧桌面 + 回滚服务/数据|配置/本地库向前兼容策略和不可降级提示明确；不把新 schema 写入旧服务|

备份必须覆盖实际 DB 后端快照、配置/脚本/订阅、加密密钥保管、持久 /data 与 tasks.json/已下载文件、外部服务版本和反代配置；凭据不进文档。Redis/Kvrocks、Upstash、D1/非 CF SQLite、Postgres、Turso 各用自身一致性备份策略。localstorage 不具有完整服务端同步，不能当完整 DB。先核对应用导出接口包含的全部字段；加密导出不是未经审计即可信赖的唯一灾备。

迁移采用 expand/contract 与执行台账，失败即停、单独批准、可在副本回放。015 等删除性迁移不能靠回退容器恢复表。回滚流程必须先停写/协调任务，恢复同时间点 DB/文件/服务版本，校验账号与收藏历史/任务，再恢复流量；没有经过副本恢复验证就不声称具备回滚能力。桌面启动/升级不执行生产 DB 迁移。

## CI 的实际风险与最小改动提案

精确证据见 .github/workflows/cloudflare-deploy.yml:3–7、:185、:189–216、:220–259：

- main push 自动触发，当前没有 paths/paths-ignore；workflow_dispatch 也存在；没有看到受保护环境门禁。
- D1 模式会遍历 migrations/*.sql，远程执行，错误用 echo 吞掉后仍可继续部署。单纯桌面/文档合入也可能走到这里。
- 打印生成的 wrangler.toml 有暴露注入配置/敏感项的风险，不应在新流程继续全文输出。
- Android 更新从原仓非草稿、非预发布的 client-v* release 中生成 latest.json；桌面标签必须独立。
- HEAD 的 Docker full/lite/dev、EdgeOne、Android 与 Turso 工作流逐一列在索引；当前发布主要为手动 workflow_dispatch，并非都自动 main 发布。Docker 目标包含 ghcr.io/mtvpls 等硬编码，后续发布前须明确用户实际目标与权限。

补丁仅建议改 Cloudflare workflow：main push 的 paths-ignore 排除 apps/desktop-nsh/** 和 docs/nsh-*；deploy job 限 main 并指定 server-production；移除 wrangler.toml 全文打印。没有修改实际 YAML。较长期可采用服务器路径白名单，但必须先审完整根配置/共享依赖，避免该发的服务端变更被过滤。

**先实际配置受保护环境再合入 CI 补丁。** 单写 environment 不自动产生审批规则；环境审批/允许分支受仓库权限和套餐限制。首次补丁会改 workflow 自身，不满足路径忽略，可能触发流水线；没有有效环境保护时应先由仓库管理员暂停/改为显式手动生产发布后再合入。本轮没有执行任何设置。

|事件|提案预期|
|---|---|
|main 仅 apps/desktop-nsh/** 或本轮 docs/nsh-*|CF workflow 不触发；桌面验证按独立路径工作流执行|
|main 同时包含服务端/根依赖/实际 workflow 改动|CF 流程进入真实保护环境等待，未经授权不部署/迁移|
|开发分支/PR|只验证、测试和 artifact；无生产 secrets，无部署或远程迁移|
|非 main 手动 CF|job ref guard 跳过|
|main 手动 CF|仍受环境保护；路径过滤不限制手动触发|
|desktop-v*|只进入以后独立桌面发布，不被 client-v* Android 扫描|
|client-v* / 服务端版本|维持 Android 语义；服务端另用明确版本渠道，不与桌面混用|

开发建议分支 research/nsh-full-scope，实施再用 feat/desktop-nsh-*；本轮未切分支、未提交。PR 路径检查与发布工作流分离：始终完成的总检查聚合按路径可选任务，避免被跳过的部署工作流成为 required check 导致 PR 永久等待。PR 对不可信代码不发签名/生产凭据。安装包发布必须使用原仓、正确目标和 desktop-v 标签，提交/合并不等于上线授权。

后续桌面 CI 提案：独立 desktop-nsh-check 在 PR/push 桌面路径上运行固定 Rust 工具链、fmt/clippy/test 和 Windows 构建；只上传短期 artifact。另有显式桌面发布工作流，绑定 desktop-production 环境、Windows x64 release 构建、依赖许可证清单/版本 SHA、SHA-256、签名、安装包/便携包；先出草稿与可核对清单，再按授权发布。未来其他系统先加入真实构建与媒体验收再声明支持。此轮未创建这些工作流。

## 自动验证与安装交付设计

|层次|验证内容|不能由它推导的结论|
|---|---|---|
|纯函数/状态机与 mock 合同|单位/集号、源分组/筛选、刷新合并、错误映射、状态迁移、离线任务、字幕时轴、中继 URI/KEY/MAP/Range；fixture 从精确源合同独立整理|不能宣称真实服务或第三方接口可用|
|隔离真实服务器与存储合同|真实登录/刷新/撤销/不同角色、搜索 SSE、详情/懒解析、收藏历史/设置冲突、通知和管理员接口；按后端/版本矩阵运行|不能证明全部设备解码、听感、DPI、原生焦点或安装器|
|原生媒体和人工目标机|真实音频/画面、seek/切源/连播、HLS AES/fMP4/字幕弹幕、全屏键盘、多 DPI/多屏、休眠恢复、外部播放器/DLNA 等|不能用一个系统样本代替所有桌面/手机平台|
|包与升级恢复|干净机可安装/启动/卸载、签名/hash/版本正确、旧版本数据升级/回滚、实例账号隔离与密钥库、磁盘不足与部分下载恢复|调试 ZIP 或开发机成功不是完整安装/更新验证|

服务端现有 Jest/TypeScript/构建检查与 Rust 门禁分开，不在此轮安装或执行上游脚本。媒体自动测试使用受控/有权样本，敏感真实 URL 不写入 fixture。验收报告逐项带需求 ID、客户端/服务端 SHA、OS/DPI、存储、真实或 mock、实际结果及失败边界；人工音画验证保留单独证据。

## 危险入口与本次研究限制

只读源码也能识别写作用：GET /api/admin/reset 会写默认配置，GET reload 清缓存，favorites/playrecords GET 可能迁移旧数据，cron GET 会刷新/清理/通知，脚本源详情和解析可执行管理员 JS；不是所有 GET 都安全。本轮没有请求这些 API。数据导入先清空现有数据；DELETE favorites 缺 key 且没有非空 keys 时会清空全部，原生批量空选择不能直接发送该请求。重置、清空收藏/历史、设备撤销、token 重置、webhook 设置、测试邮件/TG 都只列需求，未执行。

磁力 /api/acg 及音乐/漫画/求片/网盘等是当前路由树缺失或已下线候选；不是可以继续调用的旧路由。当前 /api/detail、source-detail 和个人/全局 TVBOX 合同仍有用途，源码不足以支持把它们统称已弃用；后续若退役必须另提兼容计划。

README 宣称、静态代码存在、参考工程模拟、本机用户测试、真实联调五类证据分别处理。文件索引的自动归类只是定位辅助；矩阵是本轮功能归纳，不能代替未来逐项合同与可用性验收。没有对全部数据分支/依赖许可证/第三方上游做运行证明，也没有生产安全审计结论。

## 下一阶段可交付结果与少量选择

下一轮若进入实施，建议先交：P0 子门任务表/原生交互草图、固定依赖的同仓最小 Windows 原生构建、按本仓合同编写的脱敏 fixture 与真实兼容报告模板、隔离 Linux 副本的版本/备份/回滚说明。随后才是实际登录→搜索→选集播放→收藏/历史同步闭环；CI 变更、服务端新增接口和任何生产操作均单独审阅，不能由本研究自动授权。

需要用户后续选择的只有：

1. 首发桌面系统：建议先 Windows x64，与现有亲测证据一致；其他桌面系统保留扩展范围，未经实测不列首发支持。Linux 服务端独立运行不受此选择影响。
2. “原来的全部”是否包含当前 HEAD 已删的历史音乐/书籍/漫画/私人影库/追番/网盘/求片，以及残留磁力四源：建议当前 HEAD 全部先等值，历史恢复列独立补齐范围；如要求恢复，请给目标历史版本或希望保留的能力集合，不必改变现有当前功能范围。
3. 后续真实联调使用哪个隔离 Linux 服务版本与存储后端：建议复制实际部署的版本和后端、使用专用测试账号/源；此轮无需提供密码，不以生产数据做重置/导入测试。

以上选择不阻塞本轮研究文档交付。架构与流程笔记保持 proposed，功能/CI/部署均未落地。

## 本轮文档校验

[静态校验记录与交付文件哈希](nsh-research-validation.json)记录唯一需求编号、CSV 列数与行数、全部 API 归属、源码证据行范围、文档链接、只读原生参考哈希与补丁可应用性。两篇 proposed 笔记已通过插件目录/格式/归档封印校验。git diff --check 通过，HEAD 未变；这些是文档与提案校验，不是原生功能或真实服务验收。
