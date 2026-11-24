# 原仓 Rust + NSH 全量需求矩阵（研究稿）

基线：`Happier-X/MoonTVPlus`，`main`，`5b132342e7d36282e67edd73d188468d8c446ac6`。日期：2026-11-24（Asia/Shanghai）。配套阅读：[路线图与模块摘要](nsh-rebuild-roadmap.md)、[逐文件基线索引](nsh-baseline-inventory.md)。本表是需求范围和静态证据，不是运行验收报告。

## 读表约定

- 稳定编号 `NSH-模块-序号` 不因排期变化重编；一行内的分号是该能力必须覆盖的细分动作。后续拆分使用子编号，不删除父项。
- 当前平台：W=普通网页，T=WebTV/Android TV 浏览器壳，A=同仓 Android KMP，N=同仓 Android NSH，K=Compose Desktop，S=服务端，O=运维。平台只表示找到了该侧实现入口，不保证其他端具有同等细节。未逐项验证 A/N 的功能一律不自动标上。
- 状态只评估目标 **Rust + NSH 桌面端及其配套交付**：M=已原生本地模拟（有参考工程演示/本机证据，真实待联调）；R=真实待联调（有参考适配代码，未实测真实原仓）；U=未做。M/R 都不算真实兼容通过。源码静态存在也不等于上游实际在线可用。
- P0=首版基础与真实接口播放闭环；P1=主要日常体验；P2=完整功能等值（包含管理与运维）；P3=媒体/设备重扩展。优先级仅代表先后，**不是删除范围**。`条件` 表示需要依赖或范围选择，不能当作永久延期。
- `契约✓(隔离实例)` 表示用本仓服务端代码 + 一次性 SQLite 库 + 受控片源在 127.0.0.1 上跑过客户端真实请求链（见 apps/desktop-nsh/docs/p0-contract-2026-11-24.md）。它不是生产实例验收，状态字母仍按 M/R/U 原义保留。
- 证据中的路径均相对本仓根目录，冒号后为上述 SHA 的行号；参考工程用绝对路径。代码优先于 README；行号可与基线索引哈希复查。每项验收是将来应达到的结果，本轮没有执行这些动作。
- 表中“复用”指调用/保留服务端契约，NSH 交互与实现仍需新写；不意味着复制旧网页/Android 源码。复杂度 S/M/L/XL 是相对规模，不承诺工期。

## 基础、账号与权限

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-BAS-01|连接实例；校验地址；显示站点/版本/开关|src/app/api/server-config/route.ts:11; C:/project/MoonTVPlus-NSH/src/api.rs:227|W/A/N/S|实例设置、连通/不兼容/重试状态；隔离账号资料|已有 server-config；建议新增版本化 capabilities，现有 Version 不能代替协议版本|HTTPS、地址含子路径|真实实例读配置，错误地址不进入假首页，切实例不串数据| P0 / R / M，契约✓(隔离实例) |
|NSH-BAS-02|导航、返回、搜索焦点、窗口与全屏|src/components/PageLayout.tsx:1; C:/project/MoonTVPlus-NSH/docs/product-v0.2.md:25|W/T/A/N/K|全新 NSH Shell、焦点/快捷键/窄窗、明暗主题与可读文本|无新增必要|NSH 窗口/可访问性|首次启动、键盘编辑、返回、DPI/缩放不遮挡；全屏最终包重验|P0 / M / L|
|NSH-AUTH-01|账号密码登录；区分数据库与旧固定密码模式|src/app/api/login/route.ts:184; src/app/api/server-config/route.ts:45|W/A/N/S|按 StorageType 呈现登录；密码不持久化|已有 login 返回 token；保留 localstorage 的能力差异|BAS-01|真实数据库账号登录成功；错误密码/空字段有准确提示；旧模式不能伪造同步| P0 / R / M，契约✓(隔离实例) |
|NSH-AUTH-02|会话恢复、自动续期、退出和过期重登|src/app/api/auth/refresh/route.ts:34; src/lib/token-config.ts:4; src/app/api/logout/route.ts:1|W/A/N/S|系统凭据库、单次刷新合并、请求重放、退出清理中继|已有 refresh/logout；4h access、60d refresh、10min 阈值|AUTH-01、原生安全存储|真实过期刷新一次后继续播放；撤销后重登；凭据不进日志| P0 / R / L，契约✓(隔离实例)；凭据仅内存，系统凭据库未做 |
|NSH-AUTH-03|注册；邀请码；默认用户组；冲突提示|src/app/api/register/route.ts:46; src/app/register/page.tsx:197|W/A/S|原生注册表单、站点禁用说明、返回登录|已有注册/邀请/默认组校验|AUTH-04、站点开关|开关关闭不显示可提交入口；邀请码错误/重复名均明确，成功可登录|P1 / U / M|
|NSH-AUTH-04|登录和注册 Turnstile 人机验证|src/app/api/login/route.ts:273; src/lib/admin.types.ts:58; apps/android-phone/app/src/main/java/com/moontvplus/phone/feature/login/TurnstileGate.kt:1|W/A/S|系统浏览器验证与回传，不用网页替代主界面|现有 turnstileToken；需新增一次性原生登录 handoff/兑换合同，不能关闭站点校验绕过|站点域名、回调机制|开启验证的真实站点可登录；过期/错误回传拒绝；无浏览器时说明条件|P0 / U / L，条件|
|NSH-AUTH-05|OIDC 登录、自动发现、补注册与信任等级限制|src/app/api/auth/oidc/login/route.ts:8; src/app/api/auth/oidc/callback/route.ts:115; src/app/api/auth/oidc/complete-register/route.ts:115; src/app/api/admin/oidc-discover/route.ts:8|W/S|系统浏览器授权、一次性回传、补用户名/邀请码|现有网页 Cookie 流程；新增桌面安全兑换，保留 state 和信任等级判定|AUTH-04 handoff、OIDC 提供商|取消/重放/错 state 不登录；成功绑定正确账号；不把 refresh token 放 URL|P1 / U / L，条件|
|NSH-AUTH-06|二维码登录：创建、显示、轮询、确认、取消、过期|src/app/api/auth/qr/create/route.ts:7; src/app/api/auth/qr/confirm/route.ts:8; src/app/tv/login/page.tsx:21; src/app/qr-login/page.tsx:20|W/T/S|原生二维码与确认信息、有效期、取消与刷新|现有 qr 五路由和会话存储；桌面角色适配后复用|已登录确认端、qr store 的部署能力|仅确认指定设备；过期 410/取消后不登录；并发轮询不串会话|P1 / U / M|
|NSH-AUTH-07|Telegram 登录/注册、深链与状态轮询|src/app/api/telegram/login/create/route.ts:12; src/app/api/telegram/login/status/route.ts:10; src/app/login/page.tsx:288|W/S|打开 Bot、等待/超时/取消、完成登录|现有 Telegram 登录会话、webhook、注册开关|TG-01、站点 HTTPS|真实 Bot 确认后正确账号登录；禁用/超时显示准确|P2 / U / M，条件|
|NSH-AUTH-08|修改密码及相关会话失效|src/app/api/change-password/route.ts:13; src/lib/access-token-invalidation.ts:1|W/S|原生密码面板、错误与成功状态|已有 change-password、token 失效逻辑；以服务端角色限制为准|AUTH-02|测试账号改密后旧凭据失效；失败不清空表单或误报成功|P1 / U / M|
|NSH-AUTH-09|查看设备、重命名、撤销单台/其他会话|src/app/api/auth/devices/route.ts:17; src/app/api/auth/devices/route.ts:41; src/app/api/auth/devices/route.ts:68; src/components/DeviceManagementPanel.tsx:1|W/S|设备列表、当前设备标识、撤销确认、即时刷新|已有 GET/DELETE/POST devices|AUTH-02|撤销指定测试设备使其失效，不影响未选择设备|P1 / U / M|
|NSH-AUTH-10|遵守用户/管理员/站长、封禁、用户组和片源权限|src/lib/permissions.ts:34; src/lib/feature-permissions.ts:1; src/lib/config.ts:854; src/app/api/admin/user/route.ts:42|W/T/S|能力入口与禁止态；401 与 403 分开，不误清合法会话|已有按源授权、组权限并集；ai_ask/live/web_live；需补客户端完整可见 capabilities|AUTH-01、ADM-01|不同组账号看到正确源/功能；直接请求越权也被服务端拒绝| P0 / R / L，契约✓(隔离实例) |
|NSH-AUTH-11|处理失败次数封禁、429 和未配置密码警告|src/lib/login-fail2ban.ts:1; src/app/api/login/route.ts:186; src/app/warning/page.tsx:1; src/middleware.ts:27|W/S|倒计时/Retry-After、可恢复错误、安全配置提示|已有 fail2ban、middleware 警告与登录限制|AUTH-01|不无限自动重试触发封禁；区分站点配置错误/账号错误| P0 / R / M，契约✓(隔离实例) |

## 发现、推荐、分类与搜索

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-DIS-01|首页热门电影/剧集/综艺；轮播与继续观看|src/app/page.tsx:232; src/components/UserMenu.tsx:309|W/T/A/N|原生模块布局、骨架、独立失败重试与续播卡片|复用 Douban/TMDB 数据；TX banner 如需客户端现有直连需适配|AUTH-10、DIS-02、LIB-03|真实数据各模块可独立加载，单模块失败不整页空白；演示明确标记|P1 / U / L|
|NSH-DIS-02|电影热门/最新/高分/冷门、地区分类与分页|src/components/DoubanSelector.tsx:55; src/app/api/douban/categories/route.ts:25; src/app/api/douban/route.ts:18|W/T/A/N/S|原生分类页、筛选记忆、翻页/重试|已有 douban/categories 与 douban；参数按实际 kind/category/type 分流|豆瓣可达、CFG-02|每个分类组合与网页同数据语义；末页无重复；失败不伪造评分|P1 / U / M|
|NSH-DIS-03|剧集/综艺/番剧/剧场版多级筛选：类型、地区、年份、平台、标签、排序|src/components/MultiLevelSelector.tsx:17; src/app/api/douban/recommends/route.ts:28; src/app/douban/page.tsx:353|W/S|原生多级选择、重置和筛选摘要|已有 recommends category/format/region/year/platform/sort/label|DIS-02|逐种内容类型可选值与实际请求相符，清空筛选恢复默认，分页保留条件|P1 / U / L|
|NSH-DIS-04|动漫每日放送：按星期切换；番剧/剧场版|src/components/DoubanSelector.tsx:109; src/app/api/bangumi/calendar/route.ts:6; src/app/douban/page.tsx:324|W/T/A/N/S|星期切换、名称中日文降级、原生卡片|现有 Bangumi calendar/subject/v0 subjects；复用数据源配置|CFG-02|周一至周日归属正确，无中文名时显示原名，未放送不显示虚假时间|P1 / U / M|
|NSH-DIS-05|动漫放送时刻表、当前季度、未知时段分组|src/app/api/bangumi/schedule/route.ts:45; src/lib/bangumi-schedule.server.ts:1; src/components/BangumiScheduleTimeline.tsx:1|W/T/S|原生时间轴/网格切换、时区和未知时间|已有 LiveChart+BGM 合并、内存/DB 缓存|DIS-04、LiveChart 可达|北京时间与星期转换正确，未知时间单独呈现，不推成已上线追番订阅|P2 / U / L|
|NSH-DIS-06|即将上映、TMDB 趋势、海报/剧照与预告|src/app/api/tmdb/upcoming/route.ts:15; src/app/api/tmdb/trending/route.ts:19; src/app/api/tmdb/videos/route.ts:21; src/components/TrailerPickerDialog.tsx:1|W/A/S|即将上映模块、预告选择/打开、图集查看|已有 TMDB 系列路由；媒体直链/外部链接分别处理|TMDB key/代理、DET-03|无 key 时说明不可用；预告非媒体 URL 不误当可播媒体|P1 / U / L，条件|
|NSH-DIS-07|自定义分类；按配置 type/query 导航|src/components/DoubanCustomSelector.tsx:1; src/lib/admin.types.ts:107; src/app/douban/page.tsx:652|W/S|原生动态导航、选择与内容加载|已有 CustomCategories/搜索；配置由 ADM-09 维护|DIS-02、ADM-09|增删/禁用后的配置在重新拉取后正确出现，不硬编码栏目|P2 / U / M|
|NSH-DIS-08|脚本高级推荐：选脚本、分页、进入详情|src/app/advanced-recommendation/page.tsx:40; src/app/api/advanced-recommendation/videos/route.ts:12|W/S|原生脚本源列表、分页/空态、详情跳转|已有 enabled scripts/getSources/recommend；保留脚本在服务端执行|SRC-04|有 recommend hook 可列出作品；hook 失败只影响该脚本|P2 / U / M，条件|
|NSH-DIS-09|单源浏览：选源、分类层级、搜索、分页|src/app/source-search/page.tsx:212; src/app/api/source-search/categories/route.ts:1; src/lib/category-tree.ts:1|W/S|原生源选择/分类树/面包屑、源内搜索|已有 source-search 四路由，含 special 标记与脚本|AUTH-10、SRC-01|源切换清理旧分类/分页；未授权源不能读取；脚本源分类可降级|P1 / U / L|
|NSH-SRC-01|多源聚合搜索与普通结果视图；取消/重试|src/app/api/search/route.ts:19; src/components/SearchPageClient.tsx:406; C:/project/MoonTVPlus-NSH/src/api.rs:259|W/T/A/N/K/S|原生搜索、结果分组、空/部分失败/取消态|已有 search JSON；标题+年+类型聚合要按字段保真|AUTH-10、源配置|真实实例返回多个源并可点播；同名不同年不误合并；迟到请求不覆盖新查询| P0 / R / L，契约✓(隔离实例) |
|NSH-SRC-02|实时分源搜索进度、源错误、全部完成|src/app/api/search/ws/route.ts:85; src/app/api/search/ws/route.ts:159; src/components/SearchPageClient.tsx:1253|W/A/N/S|原生 SSE 解析、增量结果、取消、断流与降级|已有 /search/ws 实际是 text/event-stream，非 WebSocket；start/source_result/source_error/complete|SRC-01、流式 HTTP|首源到达即出现；分包中文/多行事件正确；单源超时不吞掉已到结果| P1 / U / L，服务端语义已验(隔离)；客户端 SSE 未接 |
|NSH-SRC-03|搜索建议/热词、搜索历史增删清空|src/app/api/search/suggestions/route.ts:13; src/app/api/searchhistory/route.ts:17; src/components/SearchSuggestions.tsx:1|W/T/A/S|原生建议列表、防抖、历史、清空确认|已有 suggestions、searchhistory GET/POST/DELETE|AUTH-02|输入变化不显示过时建议；只删除指定测试关键词；重启与其他端一致|P1 / U / M|
|NSH-SRC-04|脚本源搜索/详情/懒解析、多子源、请求头|SOURCE_SCRIPT.md:62; src/lib/source-script.ts:294; src/app/api/source-script/play/route.ts:16; src/app/api/source-detail/route.ts:109|W/S|识别 script: 源和 needResolve；解析中/失败重试；保留 headers|已有 JS hook 执行；新增或完善给原生播放器的结构化最终 URL/headers 合同|ADM-08、PLY-02|脚本未解析 URL 不直接交播放器；needResolve=false 不重复解析；headers 正确随媒体请求|P1 / U / L，条件|
|NSH-SRC-05|搜索精确匹配、源/标题/年份筛选、年份升降序、繁简转换|src/components/SearchResultFilter.tsx:7; src/components/SearchPageClient.tsx:242; src/components/UserMenu.tsx:226|W/S|原生筛选控件、聚合/非聚合各自记忆|现有结果字段；繁简转换需原生库或服务端辅助|SRC-01|未知年份置后；筛选后统计正确；切视图不串条件；繁简可关闭|P1 / U / M|
|NSH-SRC-06|缓存命中、强制刷新与单源补查|src/components/SearchPageClient.tsx:1187; src/app/api/search/one/route.ts:18; src/app/api/search/resources/route.ts:11|W/S|实例/账号隔离缓存、刷新按钮、单源重试|已有 one/resources、fresh/refresh；具体语义以代码为准|SRC-01、CFG-01|不能用部分缓存冒充完整结果；权限变化后旧缓存不泄露已禁用源|P1 / U / M|
|NSH-SRC-07|特殊源独立入口、开关与普通结果隔离|src/app/sp/page.tsx:1; src/app/under/page.tsx:1; src/lib/special-source.client.ts:1; src/app/api/search/route.ts:27|W/S|独立入口/用户显式切换、提示与持久状态|已有 special=1、SpecialSourceApis；普通入口隐藏不等于授权|AUTH-10、ADM-07|普通搜索不混入特殊源；切换后搜索/详情/单源浏览保持标记；服务端权限仍生效|P2 / U / M|
|NSH-SRC-08|磁力候选：acgrip/mikan/dmhy/nyaa 查询、资源链接|apps/android-phone/shared/src/jvmSharedMain/kotlin/com/moontvplus/phone/core/network/MoonApi.kt:89; apps/android-phone/shared/src/commonMain/kotlin/com/moontvplus/phone/core/model/AcgModels.kt:1|A 残留合同；当前 S/W 无路由|条件项：新写原生资源列表/复制链接/外部下载器；禁止承诺直接 BT 播放|当前无 /api/acg；需决定恢复后新增聚合后端/权限；不是已实现|用户确认恢复范围、第三方源/下载器|恢复时四源/分页/失败语义验收；未恢复前标明当前 HEAD 缺失，不虚构搜索成功|P2 / U / L，范围待选|

## 短剧独立体验

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-DJS-01|短剧片源选择、父子分类/平铺分类、分页、浏览恢复、短剧推荐|src/app/duanju/page.tsx:192; src/app/duanju/page.tsx:241; src/app/duanju/page.tsx:348; src/app/api/duanju/recommends/route.ts:82|W/S|原生短剧导航、分类、加载/空/失败状态、分页和返回位置恢复|现有 sources/categories/videos/recommends；推荐取首个配置源的分类结果，不声称个性化算法|AUTH-10、DIS-09、ADM-06|真实短剧源切换不串页；父子分类和页码正确；无源/超时可恢复；推荐失败不伪装成功|P1 / U / L|
|NSH-DJS-02|独立竖屏连播、滚轮/键盘/拖动切集、选集、倍速、预播、换源测速、收藏续播、批量下载、切普通播放器|src/app/duanju/play/page.tsx:744; src/app/duanju/play/page.tsx:906; src/app/duanju/play/page.tsx:1178; src/app/duanju/play/page.tsx:1376; src/app/duanju/play/page.tsx:1403; src/app/duanju/play/page.tsx:1584; src/components/UserMenu.tsx:1|W/S|NSH 竖屏媒体布局与焦点/手势；新写活动/相邻集资源管理；保留关闭竖屏偏好；历史标记 is_duanju|现有 source-detail/search、收藏/记录；复用服务端代理，下载按 DL 合同|DJS-01、PLY-01/02/05、LIB-01/03、CFG-01、DL-01/02|切集正确保存一基集号/秒进度；只当前集发声；失败可重试/换源；横竖模式续播一致；下载遵守同一任务状态机|P2 / U / XL|

## 详情与点播闭环

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-DET-01|展示标题、海报、年份、简介、类型、片源与剧集|src/app/api/detail/route.ts:15; src/app/api/source-detail/route.ts:94; src/components/DetailPanel.tsx:55; C:/project/MoonTVPlus-NSH/src/api.rs:277|W/T/A/N/K/S|原生详情/选集；HTML 简介净化、缺图/缺字段降级|detail 与 source-detail 均存在；播放取后者新鲜契约优先|SRC-01|真实源返回的集数/标题不截断，缺失资料不伪造，选中第 N 集正确播放| P0 / R / L，契约✓(隔离实例) |
|NSH-DET-02|元数据多源：豆瓣/Bangumi/CMS/TMDB，演员导演、类型、原站链接|src/components/DetailPanel.tsx:211; src/components/DetailPanel.tsx:739; src/app/api/tmdb/credits/route.ts:1|W/S|原生元数据来源标记、切换、演职员列表与外链|已有 douban/detail、bangumi/subject、tmdb/detail/credits|DIS-04、DIS-06|同名电影/剧集/年份匹配正确；显示实际来源；切换不改变当前点播作品|P1 / U / L|
|NSH-DET-03|季/集元数据、剧照、海报图集与预告|src/app/api/tmdb/seasons/route.ts:15; src/app/api/tmdb/episodes/route.ts:15; src/app/api/tmdb/images/route.ts:15; src/app/api/tmdb/episode-images/route.ts:15|W/S|原生季集/图集/预告选择，图片虚拟列表|已有 TMDB 季/集/剧照/视频路由|TMDB key、DET-02|选季不串剧集；图片分页无重复；上游限流可恢复|P2 / U / L，条件|
|NSH-DET-04|相似影片推荐；豆瓣/TMDB/Mixed 回退|src/components/SmartRecommendations.tsx:47; src/app/api/douban-recommendations/route.ts:22; src/app/api/tmdb-recommendations/route.ts:50|W/S|原生相关推荐、缓存与失败保留已有内容|已有三种模式；Mixed 才进行回退，且受评论开关影响|DIS-06、DET-02|严格模式不擅自混数据源；单源无数据说明可见，点击可找源|P1 / U / M|
|NSH-DET-05|豆瓣短评分页；AI 评论独立标记|src/app/api/douban-comments/route.ts:19; src/app/api/ai-comments/route.ts:63; src/components/DoubanComments.tsx:1; src/components/AIComments.tsx:1|W/S|原生短评列表/分页和 AI 标签|已有抓取/生成接口；AI 由服务端 key 与配置控制|AI-01、评论开关|真实短评不标成 AI 或相反；禁用不请求；分页/失败可重试|P2 / U / M，条件|
|NSH-PLY-01|原生视频：播放/暂停、进度、seek、音量、静音、结束与重试|src/app/play/page.tsx:7082; C:/project/MoonTVPlus-NSH/docs/product-v0.2.md:32|W/T/A/N|NSH Video、自绘控制栏、加载/失败/实际时间状态|已有媒体直链、代理和原生中继参考|原生媒体后端、DET-01|真实 MP4/HLS 可播有声，seek 以实际时间确认；失败重试保持进度|P0 / M / L|
|NSH-PLY-02|鉴权媒体、HLS 主/子清单、KEY/MAP、相对 URI、Range|src/app/api/proxy/vod/m3u8/route.ts:1; src/lib/hls/proxy-rewrite.ts:1; src/app/api/video-proxy/route.ts:7; C:/project/MoonTVPlus-NSH/src/relay.rs:1|W/T/A/N/S|会话级 loopback 中继、媒体 headers、令牌更新、不跨 origin 泄露|保留 /proxy/vod 与 /proxy-m3u8；审计多音轨/byte-range/redirect 支持差异|AUTH-02、SRC-04|真实 AES/fMP4/Range/多级清单可回放；源站和实例鉴权不串，退出后旧 capability 失效|P0 / M / XL|
|NSH-PLY-03|多源/多集切换、选集排序、上一集/下一集、续播对齐|src/components/EpisodeSelector.tsx:1; src/app/play/page.tsx:5543; C:/project/MoonTVPlus-NSH/docs/product-v0.2.md:33|W/T/A/N|原生侧栏、保留位置、切源恢复与取消旧请求|已有 source-detail、source ranking 输入|DET-01、PLY-01|切源匹配到相应集数/位置；新源集数不足有明确降级；迟到结果不跳回旧集|P0 / M / L|
|NSH-PLY-04|自动下一集、结束倒计时、最后一集与重播|src/app/play/page.tsx:9287; C:/project/MoonTVPlus-NSH/docs/product-v0.2.md:81|W/T/A/N|原生倒计时、取消、开关持久化|无新增必要|PLY-03、LIB-03|播完自动进入下一集；取消有效；末集不无限循环或假报完成|P0 / M / M|
|NSH-PLY-05|片源测速与优选：速度/分辨率/综合、权重、快速/完整策略|src/app/play/page.tsx:3523; src/lib/source-ranking.ts:1; src/components/UserMenu.tsx:172|W/S|原生逐源状态、超时/取消、评分与手动指定源|已有源 weight 与 URL；测速客户端新写，可追加安全 probe 能力|PLY-02、ADM-07|按相同输入排序可解释；测速失败不阻止手动播放；不会测速所有源导致卡死|P1 / U / L|
|NSH-PLY-06|直链播放；代理回退；HLS/普通媒体类型识别|src/app/play/page.tsx:5001; src/app/api/proxy-m3u8/route.ts:17; src/app/api/video-proxy/route.ts:7|W/S|原生直链输入/打开与错误回退，仅支持已声明协议|已有 directplay/代理安全校验|PLY-02、SSRF/凭据边界|直连失败可显式切代理；不形成无限回退；不把磁链当 HTTP 视频|P1 / U / M|
|NSH-PLY-07|全屏、画中画、窗口大小与播放焦点|src/app/play/page.tsx:8217; src/components/tv/player/TVNativeVideo.tsx:1; C:/project/MoonTVPlus-NSH/docs/product-v0.2.md:35|W/T/A/N|原生全屏状态机、PiP/小窗能力探测、退出后焦点恢复|无新增必要|NSH/Windows 窗口支持|F/F11/按钮/Esc 最终包逐项重验；不吞输入框按键；PiP 无能力要显示原因|P1 / M / L，PiP 未做|
|NSH-PLY-08|倍速、清晰度、音轨与字幕轨选择及记忆|src/app/play/page.tsx:1818; src/app/play/page.tsx:1891; apps/nsh-phone/README.md:83|W/A/N|原生 tracks 面板、倍速记忆；不支持的 track 明确显示|服务端提供真实多档信息；原生媒体桥需补查询/选择能力|PLY-01、NSH media bridge|切轨/清晰度保持进度；单档不显示假选项；倍速恢复准确|P1 / U / L|
|NSH-PLY-09|键盘快进/后退/音量/倍速/上下集；长按快进；可配置步长|src/app/play/page.tsx:245; src/app/play/page.tsx:6463; src/lib/player-seek-settings.ts:1|W/T/A/N|焦点感知快捷键与触控长按、设置面板|已有本地设置同步白名单 step/quickForward|PLY-01、CFG-01|文字编辑不触发播放；快进 clamp 到时长；P 与方向键分别记忆步长|P1 / U / M|
|NSH-PLY-10|片头片尾跳过：手动秒数、当前时间设置、按片保存|src/app/play/page.tsx:4585; src/app/api/skipconfigs/route.ts:11|W/A/N/S|原生编辑/启用/删除跳过配置；片尾连播|已有 skipconfigs GET/POST/DELETE|PLY-04、AUTH-02|同 source/id 多端一致；越界配置不跳死循环；不同作品不串配置|P1 / U / M|
|NSH-PLY-11|剧集标题纠正、TMDB 对应、过滤规则和过滤后选集|src/components/EpisodeTitleCorrectDialog.tsx:1; src/components/EpisodeFilterSettings.tsx:1; src/lib/episode-title-correction.ts:1; src/app/play/page.tsx:5798|W/S|原生纠正对话框/过滤规则、原始集号映射|现有标题纠正/集数过滤客户端逻辑；TMDB 元数据 API|DET-03、PLY-03|过滤不改变服务端真实 index；纠正后历史/连播/弹幕仍对应同一集|P2 / U / L|
|NSH-PLY-12|缓冲档位、下一集媒体预缓存和弹幕预载|src/components/UserMenu.tsx:214; src/app/play/page.tsx:7176|W/S|原生缓存预算、档位/预载开关、取消时释放|已有数据 URL 和弹幕 API；原生媒体后端需开放缓冲/预载能力|PLY-02、DM-01、磁盘/内存预算|高低档有实际行为差异；换片取消预载，不以按钮存在代替有效|P2 / U / L|
|NSH-PLY-13|进度缩略预览、截图、比例、画面翻转等媒体控件|src/lib/artplayer-plugin-auto-thumbnail.ts:1; src/app/play/page.tsx:7100; src/app/play/page.tsx:8494|W|原生帧提取/截图保存/画面变换，重新设计控件|无现成跨端接口；原生媒体桥/GPU 需新增帧与变换能力|PLY-01、系统媒体 API|可保存实际帧；拖动预览不打断播放；比例/翻转立即生效|P2 / U / L，条件|
|NSH-PLY-14|外部播放器 PotPlayer/VLC/MPV/MX/nPlayer/IINA；复制可播地址|README.md:24; src/app/play/page.tsx:10648|W|原生播放器检测、用户选择、OS 协议/参数启动、错误提示|保留代理可播 URL；鉴权媒体需明确短期授权交接，不能泄露长期 token|系统安装的播放器、PLY-02|Windows 已安装播放器接到正确剧集；缺失有说明；退出后失效策略明确|P1 / U / L，按平台适用|
|NSH-PLY-15|Anime4K 超分模式与 1.5/2/3/4 倍|README.md:25; src/lib/anime4k.ts:1; src/app/play/page.tsx:4350|W|原生 GPU 后处理/模式/倍数与性能反馈；不直接嵌 WebGPU 网页|无现成服务端需求；NSH 帧纹理访问与原生 shader 适配新增|GPU、媒体渲染接口、授权|开关确实改变输出并可回退；帧率/功耗/设备失败有测量，不能用缩放冒充超分|P3 / U / XL，条件|
|NSH-PLY-16|硬解、编码/封装适配、音画同步与平台媒体差异|apps/android-phone/shared/src/commonMain/kotlin/com/moontvplus/phone/playback/Playback.kt:1; C:/project/MoonTVPlus-NSH/docs/product-v0.2.md:111|A/N；Rust 仅 Windows 本机样本|原生媒体能力探测、错误码与回退、设备选择适配|保持媒体代理；不擅自新增转码服务器|NSH/OS codec、授权测试媒体|发布声明中的系统/编码均有实机数据；不把 metadata 分辨率当实际渲染证明|P3 / U / XL，基础 HLS/MP4 归 P0|

## 字幕、弹幕、去广告与下载

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-SUB-01|源字幕和本地 SRT/VTT 等文本外挂、开关、字号|src/lib/subtitle-converter.ts:1; src/app/play/page.tsx:2734; apps/nsh-phone/README.md:120|W/A/N|文件选择、时间轴字幕层、轨道/样式面板|保留源字幕 URL/headers；无需上传本地文件|PLY-08、字体|文本时间/编码正确；切集清理旧字幕；关闭后无残留|P1 / U / L|
|NSH-SUB-02|ASS/SSA 完整样式与特效|src/app/play/page.tsx:2802; package.json:52; apps/nsh-phone/README.md:205|W；A/N 仅基础解析|原生 libass 或等值渲染桥、字体资源管理；不能以纯文本降级算等值|无新增必要|原生字幕库/许可、PLY-13|定位、描边、字体、动画样本一致；降级明确标记，不声称复杂样式通过|P2 / U / XL，条件|
|NSH-SUB-03|PGS 图像字幕、源内/外挂切换|src/app/play/page.tsx:2700; src/app/play/page.tsx:2900|W|原生 PGS 解码叠加、切轨生命周期|源媒体/字幕数据需可访问，保留鉴权|PLY-02、原生媒体接口|实际位图字幕正确显示/seek；销毁切轨释放资源，不强行转为文本|P3 / U / XL，条件|
|NSH-DM-01|弹幕自动匹配、手动搜索、选番/选集、加载与切集|src/app/api/danmaku/match/route.ts:9; src/app/api/danmaku/search/route.ts:9; src/app/api/danmaku/episodes/route.ts:9; src/app/api/danmaku/comment/route.ts:35|W/A/N/S|原生匹配选择面板、按集状态、失败重试与播放器时间同步|现有四路由、builtin/custom 后端|弹幕后端、AUTH-02|真实上游可用时随集加载；503 明确显示，不用 mock 弹幕冒充成功|P1 / U / L，条件|
|NSH-DM-02|弹幕开关、透明度、字号、速度、区域和类型屏蔽、繁简转换|src/components/DanmakuPanel.tsx:1; src/app/play/page.tsx:7410; src/lib/danmaku/traditional-to-simplified.ts:1|W/A/N|原生排版/绘制层、碰撞与同步、设置持久化|现有 comment 数据；不依赖网页插件|DM-01、NSH Canvas/文本|滚动/顶/底多类型正确，seek 不堆叠旧弹幕；控制栏仍可点击|P1 / U / L|
|NSH-DM-03|弹幕关键字/正则过滤规则、启停/编辑、云端保存|src/components/DanmakuFilterSettings.tsx:1; src/app/api/danmaku-filter/route.ts:11; src/lib/types.ts:283|W/S|原生规则编辑器、非法正则提示、安全匹配|已有 GET/POST 过滤配置；与本地临时过滤分开|DM-02、AUTH-02|过滤前后数量可解释；规则重启/换端保持，坏规则不使播放器失去响应|P2 / U / M|
|NSH-DM-04|弹幕热力图、进度定位、最大数量、预载/缓存清理|src/app/play/page.tsx:8735; src/components/DanmakuCacheCleanup.tsx:1; src/lib/danmaku/cache.ts:1; src/components/UserMenu.tsx:223|W/S|原生热力图、缓存占用/清理、预算与下集预载|已有数据，缓存主要属客户端|DM-01、PLY-12|换到无弹幕集清除上一集曲线；清理范围只限用户选择缓存|P2 / U / L|
|NSH-AD-01|内置 HLS 切片去广告、开关与播放连续性|src/lib/hls-ad-filter.client.ts:165; src/app/api/proxy-m3u8/route.ts:17; src/lib/__tests__/hls-ad-filter.test.ts:1|W/A/N/S|原生播放使用明确过滤后的清单、状态与诊断|现有服务端代理过滤；原生 UA 要与源配置匹配|PLY-02、ADM-07|过滤样本移除目标段，KEY/MAP/断点和 A/V 同步仍正确；关闭回原流|P1 / U / L|
|NSH-AD-02|自定义去广告代码、版本更新与回退|src/app/api/ad-filter/route.ts:15; src/lib/hls-ad-filter.client.ts:189; src/lib/admin.types.ts:52|W/S|原生设置/结果反馈，不直接在 Rust 进程执行站长 JS|现有客户端 JS 功能；需服务端提供等值受控过滤执行/结构化结果，管理编辑见 ADM-17|AD-01、脚本执行隔离|真实自定义规则版本生效且失败可回退；不把内置过滤当自定义代码已支持|P2 / U / XL，条件|
|NSH-DL-01|本机 M3U8 下载：选集、TS/MP4、范围/AES、合并保存|src/lib/m3u8-downloader.ts:38; src/components/DownloadEpisodeSelector.tsx:1; src/components/DownloadPanel.tsx:1|W|原生下载队列、文件选择、解密/合并、进度；浏览器三种存储映射到本机文件/缓存|现有媒体代理；下载引擎需原生新写|PLY-02、文件权限/空间|选中剧集生成真实可播文件；AES/Range 样本验证；取消不留假完成任务|P1 / U / XL|
|NSH-DL-02|下载管理：并发/线程/超时、暂停、恢复、重试失败段、取消、日志|src/lib/m3u8-downloader.ts:245; src/lib/m3u8-downloader.ts:382; src/contexts/DownloadContext.tsx:1; src/components/DownloadManagementPanel.tsx:1|W|原生持久队列、断点记录/校验、失败段日志和重启恢复|无需把本机下载搬服务端|DL-01、CFG-01|中断后只续缺失段，重试不重复写已完成段；磁盘不足可恢复|P2 / U / L|
|NSH-DL-03|本地缓存播放优先、缓存库/目录管理、无网播放|src/lib/indexeddb-video-cache.ts:1; src/lib/m3u8-downloader.ts:1000; src/app/play/page.tsx:3941|W|原生离线库、目录访问重授权、完整性检查、缓存命中提示|服务器缓存与本机缓存分开；不把浏览器 IndexedDB 直接当原生库|DL-01、LIB-03|断网可播已完整缓存剧集；缺段明确提示；本地播放继续记录进度|P2 / U / L|
|NSH-DL-04|服务器离线下载：创建/列表/进度、重试、删除与重启恢复|src/app/api/offline-download/route.ts:13; src/app/api/offline-download/route.ts:103; src/lib/offline-downloader.ts:107|W/S/O|原生管理员任务面板、操作结果/危险删除确认|现有 GET/POST/PUT retry/DELETE，仅 admin/owner、文件系统 /data/tasks.json；普通用户不开放|Linux 持久磁盘、站点开关、ADM-01|授权测试环境创建/重启后暂停/重试/删除行为准确；云边缘不可用时说明原因|P2 / U / L，条件|
|NSH-DL-05|播放服务器已下载清单/分片，自动命中和鉴权|src/app/api/offline-download/local/route.ts:35; src/app/api/offline-download/local/[source]/[videoId]/[episodeIndex]/[...file]/route.ts:36; src/app/play/page.tsx:3615|W/S|原生远端缓存标记、切回在线/重试|已有 local 两种取文件路由与 check；保持权限/路径限制|DL-04、PLY-02|正确 source/id/episode 命中；无权或非法路径被拒绝；不能误称客户端已离线|P2 / U / M，条件|

## 直播、电视与设备

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-LIVE-01|电视直播：选 M3U 源、分组/频道/搜索、台标、切台|src/app/live/page.tsx:412; src/app/api/live/sources/route.ts:10; src/app/api/live/channels/route.ts:8|W/T/S；A 使用网页入口|原生频道页/列表、切台取消、失败重试|已有 live sources/channels 与 live 权限|直播源、PLY-02、AUTH-10|真实 M3U 频道可播、分组/中文名准确，快速切台不回旧台|P1 / U / L|
|NSH-LIVE-02|EPG 当前/后续节目、XML 下载解析与节目时间|src/app/live/page.tsx:673; src/app/api/live/epg/route.ts:8; src/app/api/live/epg/download/route.ts:5|W/T/S|原生节目单、当前节目高亮、时区显示|现有 epg、epg/download、XML/压缩数据处理|LIVE-01、EPG 上游|跨日/时区正确，缺 EPG 不阻塞频道播放|P1 / U / M|
|NSH-LIVE-03|直播测速/预检、自动线路选择、三种代理模式|src/app/live/page.tsx:2141; src/app/api/live/precheck/route.ts:11; src/lib/admin.types.ts:124|W/T/S|原生逐台/线路测速、超时和 full/m3u8-only/direct 状态|已有 precheck、proxy/m3u8/key/segment/logo、源 UA|LIVE-01、ADM-10|三种模式各用真实样本；超时可手动选台；源 UA 正确、不乱泄 token|P1 / U / L|
|NSH-LIVE-04|直播收藏、最近频道、可选历史同步|src/app/live/page.tsx:543; src/app/tv/live/page.tsx:34; src/components/UserMenu.tsx:169; src/lib/types.ts:16|W/T/S|原生常用频道、续看、保存历史开关|已有 Favorite/PlayRecord origin=live|LIB-01、LIB-03|直播记录不被点播续播误识别；默认开关遵循用户选择|P1 / U / M|
|NSH-LIVE-05|网络直播：虎牙/Bilibili/抖音房间、开播状态、FLV/HLS|src/app/api/web-live/stream/route.ts:209; src/app/web-live/page.tsx:298; src/app/api/web-live/proxy/[filename]/route.ts:41|W/S|原生房间列表、播放/未开播、刷新临时地址、FLV 能力回退|现有三平台抓流/代理、web_live 权限；不是任意平台支持|ADM-11、原生 FLV/HLS、上游反爬|每平台实际样本；未开播不误报鉴权失效；FLV 不支持时给出具体原因|P2 / U / XL，条件|
|NSH-TV-01|大屏界面/遥控焦点、分类、搜索、详情、点播、个人中心|src/app/tv/layout.tsx:1; src/app/tv/play/page.tsx:1; src/components/tv/TVLayout.tsx:1; apps/android-tv/README.md:1|T|重新设计 NSH 大屏模式与方向键焦点，保留原 TV 壳过渡|复用同一 API，ENABLE_TV_MODE 开关|BAS-02、PLY-03、设备输入|方向键可完成完整找片/播放/返回；不可达按钮为零；无鼠标也可用|P3 / U / XL|
|NSH-TV-02|手机/桌面虚拟遥控器：发现设备、方向键/数字/文本输入|src/app/api/tv-remote/devices/route.ts:10; src/app/api/tv-remote/key/route.ts:11; src/app/api/tv-remote/text/route.ts:11; src/lib/tv-remote-types.ts:1|W/T/S|原生设备列表、遥控面板、文本发送和接收适配|已有 tv-remote hub/HTTP/Socket，保留设备/账号边界|TV-01、局域网/服务端连接|只控制选中设备；replace/append/backspace/clear 正确；离线不假报已送达|P2 / U / L|
|NSH-TV-03|TVBOX 订阅、个人 token 获取/重置、去广告/内容过滤参数|src/app/api/tvbox/subscribe/route.ts:17; src/app/api/user/tvbox-token/route.ts:13; src/app/api/user/tvbox-token/reset/route.ts:13; src/components/UserMenu.tsx:491|W/S/O|原生电视访问面板、复制/二维码、重置确认|现有个人 token 和旧全局 TVBOX_SUBSCRIBE_TOKEN 两条合同；保留 blocked sources/SITE_BASE|AUTH-10、ADM-07|个人链接只含获准源；重置旧链接失效；不能把全局链接当个人权限安全|P2 / U / L|
|NSH-TV-04|OrionTV/兼容 App 后端与记录同步|README.md:631; src/app/api/source-detail/route.ts:174; src/lib/downstream.ts:1|S/外部客户端|NSH 显示兼容访问说明与连接信息，不冒充替代外部 App|保持既有 source/detail/playrecords/Bearer/UA 兼容|PLY-02、LIB-03|服务端演进后现有外部客户端在隔离测试仍能登录/播放/同步|P2 / U / L|
|NSH-TV-05|DLNA 投屏发现、播放、暂停、seek 与设备状态|apps/android-phone/shared/src/jvmSharedMain/kotlin/com/moontvplus/phone/cast/DlnaClient.kt:1; apps/android-phone/shared/src/commonMain/kotlin/com/moontvplus/phone/cast/DlnaProtocol.kt:1|A|原生 SSDP/UPnP、投屏设备面板、媒体鉴权交接|现有网页以浏览器投屏为主；新增桌面局域网媒体中继可达策略|局域网设备/防火墙、PLY-02|真实接收端可取媒体；loopback 不当成电视可访问 URL；断连恢复与停止正确|P3 / U / XL，条件|
|NSH-TV-06|移动端手势、PiP、通知栏/锁屏/后台播放与方向控制|apps/nsh-phone/README.md:114; apps/nsh-phone/README.md:138; apps/android-phone/shared/src/commonMain/kotlin/com/moontvplus/phone/playback/PlayerPlatform.kt:1|A/N|按桌面平台映射媒体键/系统媒体会话；未来 NSH 手机宿主另做手势与生命周期|无新增必要|设备平台、原生宿主|桌面媒体键可控制当前播放；手机扩展需真实设备验收，不宣称 Windows 自动等同 Android|P3 / U / XL，平台条件|
|NSH-TV-07|Android 安装、签名更新、站点镜像和版本验证|apps/android-phone/README.md:78; apps/android-phone/app/src/main/java/com/moontvplus/phone/update/ApkVerifier.kt:1; cloudflare-worker.ts:28|A/N/S/O|保留既有 Android 发布；未来 NSH 手机实现需等值升级流程；桌面更新另归 OPS-08|已有 /downloads/latest.json、APK 同域转发；隔离 client-v 标签|Android 签名/权限、OPS-07|既有 Android 不因桌面包被识别成更新；签名/hash/versionCode 校验继续有效|P2 / U / L|

## 片库、同步、设置与通知

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-LIB-01|收藏列表、单项状态、新增/取消与本机保留|src/app/api/favorites/route.ts:18; src/app/api/favorites/route.ts:83; C:/project/MoonTVPlus-NSH/src/api.rs:307|W/T/A/N/K/S|原生收藏页/卡片动作、区分本机/实例已确认|现有 source+id key、favorite 字段、success 回执|AUTH-02、DET-01|真实账号增删后另端可见；请求失败不标成云端已保存；断网本机操作显示未同步| P0 / R / L，契约✓(隔离实例) |
|NSH-LIB-02|收藏分组/筛选/排序、完结与更新标记、批量删除/清空|src/components/FavoritesPanel.tsx:1; src/app/api/favorites/route.ts:145; src/lib/types.ts:25|W/S|原生片库筛选、批量选择与危险动作确认|已有批量 keys/单 key/无 key 清空语义；元数据完成标记|LIB-01、AUTH-10|点播/直播分开；指定批量只影响选中项；空选择绝不发无 key DELETE| P1 / R / M，契约✓(隔离实例) |
|NSH-LIB-03|播放历史、续播、进度上报和多端同步|src/app/api/playrecords/route.ts:11; src/app/api/playrecords/route.ts:59; src/lib/types.ts:4; C:/project/MoonTVPlus-NSH/docs/product-v0.2.md:109|W/T/A/N/K/S|原生历史、稳定上报/合并、秒/毫秒转换、1 基集号、离线队列|已有 key/record 合同；参考 Rust 当前只有本机历史，服务端未接入|PLY-01、AUTH-02|真实跨端续播到同一集/秒，退出重开保持；零时长/错误不覆盖有效进度| P0 / R / L，契约✓(隔离实例)；合并规则已实现，并发冲突未验 |
|NSH-LIB-04|历史单条/批量删除、清空；新集数提醒与旧记录清理|src/app/api/playrecords/route.ts:129; src/components/PlayRecordsPanel.tsx:1; src/lib/episode-freshness.ts:1|W/S|原生删除确认、更新徽标、可恢复失败态|已有历史删除与 cleanup；保留 is_anime/is_duanju/origin|LIB-03、OPS-04|只删选择记录；新集标记不把预告/无效集当正片；不对生产做研究性删除|P1 / U / M|
|NSH-CFG-01|本地偏好：搜索/优选/直播历史/缓冲/预载/字幕弹幕/下载/跳转步长|src/components/UserMenu.tsx:168; src/lib/local-settings-sync.ts:13|W/A/N|原生分组设置、类型校验、恢复默认与版本迁移|不把本机偏好默默写入管理员全局配置|各能力模块|每个已声明开关有实际效果，重启保持；未做项不提供假可用控件|P1 / U / L|
|NSH-CFG-02|数据/图片直连、服务器代理、CDN、自定义与备用源|src/components/UserMenu.tsx:180; src/lib/douban.client.ts:1; src/lib/bangumi.client.ts:1; src/lib/tmdb-image-base.ts:1|W/S|原生数据源/图片设置、连接失败时按用户策略回退|已有豆瓣/Bangumi/TMDB proxy 与 image-proxy；桌面无 CORS 仍需尊重 headers/配置|DIS-02、DET-02|主备切换可见；图片 403 有降级；不直接外传账号凭据|P1 / U / L|
|NSH-CFG-03|设置云备份/恢复/自动同步、模式与冲突提示|src/app/api/local-settings-sync/route.ts:46; src/app/api/local-settings-sync/route.ts:94; src/lib/local-settings-sync.ts:60|W/S|原生白名单映射、手动备份恢复、自动拉取/合并、冲突 UI|现有 version=1/64KiB/MD5 去重、409 分支；写入是否 CAS 必须另验，不承诺已有强冲突保护|AUTH-02、CFG-01|只同步可迁移设置；不上传凭据/本机目录权限；并发行为有真实合同测试|P2 / U / L|
|NSH-CFG-04|首页模块开关/顺序、继续观看开关、背景与明暗主题|src/components/UserMenu.tsx:309; src/lib/admin.types.ts:135; src/components/ThemeProvider.tsx:1|W/A/N/K|NSH 主题/模块拖排与键盘排序；背景图/图标映射|已有 site theme 与本地 homeModules；CSS 不能直接作用原生控件|DIS-01、ADM-13|禁用模块不请求无用数据；排序重启保持；明暗与缩放可读|P2 / U / L|
|NSH-CFG-05|本机缓存占用、弹幕/推荐/搜索清理、版本/更新/错误提示|src/components/UserMenu.tsx:376; src/components/VersionPanel.tsx:1; src/components/GlobalErrorIndicator.tsx:1; src/lib/startup/cacheCleanup.ts:1|W|原生缓存管理、日志导出脱敏、版本信息与可恢复错误|现有版本查询/静态 CHANGELOG；不默认新增服务端诊断暴露|BAS-02、OPS-08|清缓存不删收藏/会话/离线文件；版本对应真实构建 SHA|P1 / U / M|
|NSH-NTF-01|站内通知：未读数、列表、标已读、删除/清空|src/app/api/notifications/route.ts:17; src/app/api/notifications/route.ts:45; src/components/NotificationPanel.tsx:1|W/S|原生通知中心、未读角标、跳到作品与批量确认|已有 mark_read/delete/clear_all|AUTH-02、LIB-01|真实未读数随动作更新；无权限不读取他人通知；点击能找对应作品|P1 / U / M|
|NSH-NTF-02|收藏更新检查与站内/邮件/推送/Telegram 分发|src/app/api/cron/[password]/route.ts:28; src/lib/notification-dispatch.ts:1; src/lib/types.ts:364|S/O/W|原生展示更新原因、通知设置与处理结果|现有 cron、favorite check、分发服务；不能由桌面后台另跑生产 cron|OPS-04、LIB-01、TG-01|隔离站点新增集触发一次对应通知；不开通渠道不调用；重复 cron 不刷屏|P2 / U / L，条件|
|NSH-NTF-03|个人邮箱/通知偏好、SMTP/Resend 接收|src/app/api/user/email-settings/route.ts:11; src/components/EmailSettingsPanel.tsx:1; src/lib/email.service.ts:1|W/S|原生邮箱/偏好表单、保存/错误状态|已有 email-settings 与服务端邮件发送；密钥不下发普通客户端|ADM-15、NTF-02|测试邮箱收到真实更新邮件；关闭后不再接收；无邮箱提示准确|P2 / U / M，条件|
|NSH-NTF-04|浏览器 Web Push 的原生等值系统通知/订阅管理|src/app/api/notifications/push/route.ts:19; src/lib/web-push.ts:1; next.config.js:1|W/S|桌面系统通知、权限和点击路由；后台/退出到托盘策略需明确|现有 VAPID Web Push 面向浏览器，需新增原生事件传递或受控轮询；不把站内轮询当离线推送|OS 通知、AUTH-09、NTF-01|前后台/关闭权限/撤销设备分别验收；退出进程是否接收按发布合同明示|P2 / U / XL，条件|
|NSH-TG-01|Telegram 账号绑定/解绑、通知开关、Bot 命令与深链|src/app/api/telegram/bind/route.ts:14; src/app/api/telegram/webhook/[secret]/route.ts:11; src/lib/telegram.ts:1; src/components/UserMenu.tsx:252|W/S|原生绑定码/深链/刷新/解绑、偏好|现有 bind/config/webhook；Bot token 仅服务端|ADM-16、HTTPS webhook|绑定码一次使用且绑定到正确账号；解绑立即停止对应用户分发|P2 / U / L，条件|

## AI 与社交观影室

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-AI-01|AI 问片：全站/影片上下文、流式回答、历史/停止|src/components/AIChatPanel.tsx:1; src/app/api/ai/chat/route.ts:344|W/S|原生聊天面板、上下文/历史、SSE 增量与取消、错误重试|已有 ai/chat、权限与入口开关；密钥留服务端|ADM-14、AUTH-10|真实配置模型可回答并停止；文本/错误/[DONE] 不重复或丢失|P2 / U / L，条件|
|NSH-AI-02|AI 工具：联网/抓页/豆瓣/TMDB/收藏/近期、工具状态与上下文压缩|src/lib/ai-tool-agent.ts:14; src/lib/ai-tool-agent.ts:44; src/lib/admin.types.ts:187|W/S|原生工具执行状态、引用/结果、历史压缩反馈|现有 completions/responses/claude 协议与工具循环；策略和 API key 留服务端|AI-01、外部模型/搜索提供商|三协议按配置选择；仅当前用户收藏/近期；压缩不导致工具回放错配|P2 / U / XL，条件|
|NSH-ROOM-01|创建/加入/列出房间、私密密码、房主恢复、离开/清状态|src/app/watch-room/page.tsx:350; src/types/watch-room.ts:106; src/hooks/useWatchRoom.ts:1|W/S|原生房间大厅、口令/房主身份与重连状态|现有 Socket.IO internal/external、CF Durable Object；非 REST 轮询替代|ROOM-06、AUTH-02|两真实客户端可加入同房，重连保持正确房主；错密码失败；过期房间显示清楚|P2 / U / XL，条件|
|NSH-ROOM-02|多人同步点播/直播/网络直播、seek/play/pause/rate/换集换源|src/hooks/usePlaySync.ts:1; src/hooks/useLiveSync.ts:1; src/hooks/useWebLiveSync.ts:1; src/types/watch-room.ts:28|W/S|原生状态同步、漂移纠正、房员只读与房主控制|现有 play/live 事件；不同部署实现需兼容验证|ROOM-01、PLY-03、LIVE-01|真实双端连续播放/seek/变速/切集无反馈环；误差阈值由验收合同记录并测量|P2 / U / XL|
|NSH-ROOM-03|实时文字/表情聊天与浮窗|src/components/watch-room/ChatFloatingWindow.tsx:1; src/types/watch-room.ts:57; src/types/watch-room.ts:146|W/S|原生聊天浮层、滚动/未读、发送失败与房间切换清理|已有 chat:message|ROOM-01|同房双方各收到一次；换房不残留旧消息；文本不执行 HTML|P2 / U / M|
|NSH-ROOM-04|语音通话、麦克风、WebRTC 与服务器音频中转|src/hooks/useVoiceChat.ts:1; src/types/watch-room.ts:148; README.md:577|W/S|原生采集/播放、权限、静音/设备选择/回声处理与 WebRTC|已有 signaling/audio-chunk、webrtc-fallback/server-only；需桥接原生 codec/传输|ROOM-01、音频设备/网络、原生 WebRTC|双端有声、静音和断线恢复；两种策略分别验收，未测不能以聊天成功替代|P3 / U / XL，条件|
|NSH-ROOM-05|屏幕共享房间、共享音频、辅助窗口和信令|src/app/watch-room/screen/page.tsx:1; src/hooks/useScreenShare.ts:1; src/types/watch-room.ts:135|W/S|原生窗口/屏幕选择、系统授权、接收画面、停止/资源释放|已有 screen 事件与 helper-register；NSH 无浏览器 getDisplayMedia，需平台捕获/WebRTC 桥|ROOM-01、原生采集库|真实双方可收看共享；停止后采集灯/音轨关闭；不把预览图当共享实现|P3 / U / XL，条件；ROOM-01 仅依赖传输配置，不依赖本项|
|NSH-ROOM-06|观影室部署配置、外部凭证隔离、心跳与清理|src/app/api/watch-room-auth/route.ts:15; cloudflare-worker.ts:73; src/worker/watch-room-hub.ts:83; server.js:1|S/O/W|原生显示连接目标/断连状态、按服务端获取授权，不持久化共享 secret|保留 Node Socket.IO/CF DO/外部服务器三路线；建议短期用户级 ticket 替代直接交外部共享凭据|部署模式、AUTH-02|各声明支持部署可建立/重连；应用隔离 ID 生效；外部密钥不出现在日志/截图|P2 / U / XL|

## 管理、片源与全局配置（全部纳入原生管理目标）

普通网页可作为迁移期回退入口，P2 结束前以下管理动作必须有 NSH 原生入口或明确运维工具交付；不能永久以“网页继续管”关闭需求。危险写入的测试仅限独立副本。

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-ADM-01|按 owner/admin 显示管理模块、读取配置与分页用户|src/app/api/admin/config/route.ts:11; src/app/api/admin/users/route.ts:9; src/app/admin/page.tsx:13709|W/S|原生管理导航、角色门控、分页/搜索；敏感字段默认隐藏|已有 config/users；建议新增脱敏且版本化管理读取 DTO，避免整份配置落盘|AUTH-10、CFG 能力|普通账号不可进入/直调；管理员与站长可用模块匹配服务端，不缓存密钥|P0 / U / L，首版只读诊断；P2 完整管理|
|NSH-ADM-02|用户新增、封禁/解封、删除、改密|src/app/api/admin/user/route.ts:161; src/app/api/admin/user/route.ts:193; src/app/api/admin/user/route.ts:284; src/app/api/admin/user/route.ts:319|W/S|原生用户表单/搜索、逐项确认与回执|已有 add/ban/unban/changePassword/deleteUser，保留站长保护规则|ADM-01、备份|测试账号动作准确；不允许低角色修改更高角色；删除范围可审阅|P2 / U / L|
|NSH-ADM-03|授予/取消管理员、用户组增改删、批量分组、功能/片源授权|src/app/api/admin/user/route.ts:235; src/app/api/admin/user/route.ts:348; src/app/api/admin/user/route.ts:377; src/app/api/admin/user/route.ts:471|W/S|原生权限矩阵、批量差异预览、修改后刷新|现有 setAdmin/cancelAdmin/updateUserApis/userGroup/updateUserGroups/batchUpdateUserGroups|ADM-02、AUTH-10|用户单独源限制优先、组取并集、无组兼容规则都有对照用例|P2 / U / XL|
|NSH-ADM-04|管理员查看/撤销他人设备、旧用户迁移|src/app/api/admin/user-devices/route.ts:46; src/app/api/admin/user-devices/route.ts:106; src/app/api/admin/migrate-users/route.ts:10|W/S|原生目标用户设备列表、危险动作与迁移进度/结果|已有 device revoke/migrate-users；迁移可能改持久数据|ADM-01、OPS-02|测试副本只处理目标用户；迁移后角色不被意外提升、历史收藏完整|P2 / U / L|
|NSH-ADM-05|配置文件文本编辑、校验、保存；订阅 URL/代理/自动更新/立即拉取|src/app/api/admin/config_file/route.ts:12; src/app/api/admin/config_subscription/fetch/route.ts:11; src/lib/config-subscription.ts:1; src/app/admin/page.tsx:6477|W/S/O|原生配置编辑/导入、差异预览、订阅状态；不执行上游安装脚本|已有保存/订阅拉取/代理；需新增 revision/冲突保护更稳妥|ADM-01、OPS-02、外部配置源|非法 JSON 不写入；订阅拉取失败不清空有效配置；手动配置覆盖策略有提示|P2 / U / XL|
|NSH-ADM-06|片源新增/编辑/启停/删除/排序；批量操作|src/app/api/admin/source/route.ts:89; src/app/api/admin/source/route.ts:191; src/app/api/admin/source/route.ts:289|W/S|原生片源表格/表单、批量选择、配置源与自定义来源区别|现有 add/update/disable/enable/delete/batch/sort；遵循不可删除来源限制|ADM-05、OPS-02|配置源限制与网页一致；批量部分失败逐项反馈，未选中不受影响|P2 / U / L|
|NSH-ADM-07|源验证/测速、代理模式、权重/优选、特殊源、客户端去广告源|src/app/api/admin/source/validate/route.ts:12; src/app/api/admin/source/route.ts:313; src/app/api/admin/source/route.ts:357; src/app/api/admin/source/route.ts:369|W/S|原生验证日志、权重批改、模式/名单设置|现有 toggle_proxy_mode/toggle_special_source/set_special_sources/set_client_ad_sources/weights|ADM-06、PLY-05、AD-01|逐源验证结果可追溯；NSH UA 是否命中去广告经过真实请求验证|P2 / U / L；基础诊断 P0|
|NSH-ADM-08|源脚本列表、编辑/导入/保存/启停/删除/测试各 hook、查看日志|src/app/api/admin/source-script/route.ts:42; src/app/api/admin/source-script/route.ts:79; SOURCE_SCRIPT.md:75|W/S|原生代码编辑器/导入导出、hook 参数、日志与危险执行提示|现有脚本 registry/测试/保存；new Function + require 不是安全沙箱，执行留服务端且需隔离强化|ADM-01、SRC-04、受控执行环境|五 hook 独立测试且日志/超时准确；普通客户端不执行远端脚本；取消测试有资源边界|P2 / U / XL|
|NSH-ADM-09|自定义分类新增、启停、删除、排序|src/app/api/admin/category/route.ts:18; src/app/api/admin/category/route.ts:57|W/S|原生分类表单/排序、来源标记|已有 add/enable/disable/delete/sort|DIS-07、ADM-01|type+query 标识稳定；配置来源限制与现有一致；导航拉取后更新|P2 / U / M|
|NSH-ADM-10|电视直播源 M3U/UA/EPG、启停排序、刷新间隔和手动刷新、代理模式|src/app/api/admin/live/route.ts:12; src/app/api/admin/live/route.ts:156; src/app/api/admin/live/refresh/route.ts:12; src/lib/admin.types.ts:114|W/S/O|原生直播源设置、刷新结果与频道数量|现有 live config/refresh、全局 LiveRefreshIntervalHours|LIVE-01、OPS-04|源设置/刷新只作用所选源；坏 M3U 保留可诊断状态，不误报频道存在|P2 / U / L|
|NSH-ADM-11|网络直播总开关、平台房间新增/编辑/启停/删除/排序|src/app/api/admin/web-live/route.ts:7; src/app/api/admin/web-live/route.ts:31|W/S|原生房间管理、平台参数和开播诊断|现有 WebLiveConfig/toggleEnabled/add/edit/enable/disable/delete/sort|LIVE-05、ADM-01|站点开关关闭不显示播放入口；三平台参数校验准确|P2 / U / M|
|NSH-ADM-12|站点名称/公告频率、搜索页数/缓存/流式、内容过滤、数据代理、TMDB/Bangumi/推荐与设置同步策略|src/lib/admin.types.ts:11; src/app/api/admin/site/route.ts:12; src/app/admin/page.tsx:8177|W/S|原生分组全局设置与差异预览，区分用户本地偏好|已有 SiteConfig 多字段保存；新增安全字段 DTO/版本保护建议|ADM-01、DIS、CFG|所有现存字段有映射；公告 once/every 可复现；保存不丢其他模块配置|P2 / U / XL|
|NSH-ADM-13|全局主题、CSS、背景/进度图标/加载动画/评分徽标和缓存版本|src/lib/admin.types.ts:135; src/app/api/admin/theme/route.ts:11; src/app/admin/page.tsx:6982|W/S|NSH 原生品牌/主题参数编辑与预览；网页 CSS 仍可从原生编辑器管理|现有 ThemeConfig/CSS 端点保留；新增 NSH 专属主题 schema，不能把任意 CSS 当原生样式|CFG-04、ADM-01|网页主题管理未丢；NSH 主题实际生效，平台不适用字段明确标识|P2 / U / L|
|NSH-ADM-14|AI 提供商/协议/模型/决策模型/搜索工具/上下文/入口/评论/提示词配置|src/lib/admin.types.ts:154; src/app/api/admin/ai/route.ts:12; src/app/admin/page.tsx:11403|W/S|原生 AI 管理表单、密钥只写/遮罩、模型和协议条件字段|已有 AIConfig 全部选项；需脱敏读取/未更改密钥语义|AI-01、ADM-01|三提供商/三协议配置不互相覆盖；普通用户拿不到 key；保存后实际服务可用|P2 / U / L|
|NSH-ADM-15|邮件配置 SMTP/Resend、保存与测试发送|src/app/api/admin/email/route.ts:14; src/app/api/admin/email/route.ts:89; src/lib/admin.types.ts:206|W/S|原生邮件配置、测试收件人和结果；真实发送须显式操作|现有 GET/POST save/test，密钥保留服务端|NTF-03、ADM-01、测试邮箱|隔离环境 SMTP/Resend 按所选配置发送，失败显示真实原因且不误报|P2 / U / M，条件|
|NSH-ADM-16|Telegram Bot 配置、API 代理/反代、登录注册绑定通知开关、webhook 设置和测试|src/app/api/admin/telegram/route.ts:41; src/app/api/admin/telegram/route.ts:56; src/app/api/admin/telegram/route.ts:110; src/lib/admin.types.ts:224|W/S/O|原生配置、webhook 目标预览、显式设置/测试回执|现有遮罩读取/save/set_webhook/test；不在桌面直连 Bot token 发消息|TG-01、AUTH-07、HTTPS|webhook secret 与目标正确；未操作不发送测试消息；禁用行为与开关一致|P2 / U / L，条件|
|NSH-ADM-17|自定义去广告代码的管理编辑/保存/版本更新|src/lib/admin.types.ts:52; src/app/admin/page.tsx:13827; src/app/api/admin/site/route.ts:12|W/S|原生代码编辑、版本/预览与回滚入口|已有 SiteConfig CustomAdFilterCode/Version；服务端等值执行需 AD-02 方案|AD-02、ADM-01|保存后目标版本可被原生播放使用；坏代码回退；不在普通客户端执行任意配置代码|P2 / U / L|
|NSH-ADM-18|注册/邀请码/Turnstile/OIDC 配置与默认组；流量分析配置|src/lib/admin.types.ts:58; src/lib/admin.types.ts:75; src/app/admin/page.tsx:9569; src/app/admin/page.tsx:10502|W/S|原生认证配置与 OIDC 发现；分析配置仍能管理网页，原生采集需独立明确选择|已有 site/OIDC discover；网页 analytics script 不直接在 NSH 执行|AUTH-03/04/05、ADM-01|敏感 key 只写；开关真实影响登录/注册；不把网页脚本自动注入桌面|P2 / U / L|
|NSH-ADM-19|管理配置重载、重置；原生危险操作确认与备份提示|src/app/api/admin/reload/route.ts:10; src/app/api/admin/reset/route.ts:10; src/app/admin/page.tsx:13606|W/S|原生明确危险页，必须主动点击才请求；禁止预取|现有 **GET reset 会写配置**，GET reload 清缓存；建议新增 POST/CSRF/审计并保留兼容退役计划|OPS-02、ADM-01|隔离副本验证恢复路径；普通列表/链接预览绝不调用危险 GET|P2 / U / L|

## 存储、运维、构建和发布

|编号|用户能力 / 细分动作|证据文件:行|现有平台|NSH 新写交互/实现|服务端现有/新增能力|依赖|验收结果标准|优先级 / 状态 / 复杂度|
|---|---|---|---|---|---|---|---|---|
|NSH-OPS-01|存储后端 Redis/Kvrocks/Upstash/D1/SQLite/Postgres/Turso 的配置与能力差异|src/lib/db.ts:17; src/lib/db.ts:30; src/lib/db.ts:125; VERCEL_DEPLOYMENT.md:69|S/O|NSH 诊断显示服务端后端/支持能力，不携带 DB 凭据或直接连库|已有 IStorage/adapters；SQLite 借 d1 模式在非 CF 使用；localstorage 无 DB，不能等同完整支持|Linux 独立服务、能力协议|每个声明支持后端有服务端合同测试；CF 不支持 Node Redis/Kvrocks 的限制可见|P0 / U / XL，首版选实际后端；P2 全覆盖|
|NSH-OPS-02|整站加密导出/导入、进度、跨存储迁移与灾备恢复|src/app/api/admin/data_migration/export/route.ts:17; src/app/api/admin/data_migration/import/route.ts:86; src/app/api/admin/data_migration/progress/route.ts:10; src/components/DataMigration.tsx:1|W/S/O|原生备份文件选择、口令、进度和替换范围确认；运维交付恢复手册|现有压缩加密包与 **先清空再导入**；需完整性/兼容性预检、独立副本恢复和缺失字段审计|ADM-01、完整 DB/文件快照|可在副本恢复账号/收藏/历史/设置/Telegram/推送/脚本等全部范围；格式不全不能冒称灾备|P0 / U / XL，P0 先备份/回滚设计；P2 原生全操作|
|NSH-OPS-03|DB 初始化/编号迁移、旧数据兼容与版本回滚|migrations/001_initial_schema.sql:1; migrations/015_drop_music_and_books.sql:1; scripts/init-sqlite.js:1; scripts/init-postgres.js:1; scripts/init-turso.js:1|S/O|原生只展示服务端不兼容/升级要求；迁移由运维独立执行|保留数据库后端脚本；建议 schema version/已执行记录/失败即停，禁止桌面启动自动迁移生产|OPS-02、服务版本兼容矩阵|旧服务+新桌面/新服务+旧桌面合同明确；破坏迁移需快照恢复才能降级|P0 / U / XL|
|NSH-OPS-04|定时任务：订阅刷新、直播刷新、收藏更新、缓存/历史清理|src/app/api/cron/[password]/route.ts:28; cloudflare-worker.ts:79; src/lib/startup/cacheCleanup.ts:1; wrangler.toml:1|S/O|NSH 只读状态/最近结果与原生管理入口，计划新增受鉴权任务状态 API|现有 cron 密码路由/CF scheduled/Node 启动；GET 有写作用；不由桌面自动触发|部署平台、ADM-05/10、NTF-02|隔离环境定时任务幂等、限并发/失败可见；默认密码策略单独审查|P2 / U / L|
|NSH-OPS-05|Docker/Lite/Linux 独立服务、挂载/UID/GID、外部观影室/弹幕、代理与 HTTPS|Dockerfile:1; Dockerfile.lite:1; docker-entrypoint.sh:1; README.md:274; README.md:425|S/O|原生连接/诊断；交付可审阅 Linux 部署/备份/恢复说明，非桌面携带服务端|现有 Node server.js/next、Lite 无内置观影室/SQLite 自动初始化；保留远程运行|OPS-01/02、Linux 目标环境|不升级服务也可用兼容桌面；容器重建数据不丢；反代/SSE/媒体范围正常|P0 / U / L，真实副本是 P0 前提|
|NSH-OPS-06|Cloudflare/EdgeOne/Vercel/Render 等平台部署与能力边界|.github/workflows/cloudflare-deploy.yml:3; .github/workflows/edgeone-deploy.yml:3; VERCEL_DEPLOYMENT.md:63; render.yaml:1; vercel.json:1|S/O|原生能力限制说明；运维流程重构为分平台 runbook/显式任务，非只留网页管理|已有 CF DO、EdgeOne/Turso、Vercel/Postgres；Netlify 仅 README 宣称，需另验配置|OPS-03、ROOM-06、DL-04|每个正式支持平台列实测范围；无持久盘不显示服务器离线下载为可用；不混同 CF 与 Vercel Socket 能力|P2 / U / XL，条件|
|NSH-OPS-07|同仓路径/PR/分支/tag 的验证与服务/桌面/Android 发布隔离|.github/workflows/cloudflare-deploy.yml:4; .github/workflows/cloudflare-deploy.yml:189; .github/workflows/android-nsh-release.yml:44; .github/workflows/docker-image.yml:3|O|新桌面目录独立 Cargo workspace、独立构建与包名/版本|提出 CF paths-ignore+受保护环境、桌面 CI 只构建；使用 desktop-v*，避开 APK client-v* 扫描|用户审阅 CI patch、仓库规则|桌面/docs PR 与 main 桌面路径变更不运行生产 deploy/migrate；手动发布仍需环境门禁；本轮不触发|P0 / U / L|
|NSH-OPS-08|桌面格式/静态检查/单测/合同/媒体/UI 测试、构建与安装包/更新/回滚|package.json:6; jest.config.js:1; C:/project/MoonTVPlus-NSH/README.md:37; apps/android-phone/README.md:78|W/A/N/O；Rust 仅调试 ZIP 参考|Rust fmt/clippy/test、Windows 原生构建、签名/清单/hash、解压版与安装包、用户数据迁移/回滚|新增桌面 CI 与签名发布，先 artifact 后人工发布；服务端验证流水线分离|OPS-07、NSH 固定 SHA、签名证书|干净目标机安装/更新/卸载与回滚可复现；mock/真实/人工听感报告分开；无生产 secrets 的 PR 验证| P0 / M / XL，本机构建与便携包已出；干净机/签名/安装更新未做 |
|NSH-OPS-09|PWA 安装/缓存能力的桌面等值：离线启动与资源更新|README.md:45; next.config.js:1; scripts/generate-manifest.js:1|W|原生应用资源随包、离线启动、网络恢复与资源版本；不复制 service worker|保留网页 PWA；原生媒体离线另见 DL-03|OPS-08、CFG-05|无网络仍可启动/查看本机资料；不会把 PWA 缓存宣称为所有视频离线|P2 / U / M|
|NSH-OPS-10|安全边界：代理 SSRF、header 转发、签名/设备失效、密钥/日志、脚本隔离|src/lib/server/ssrf.ts:1; src/lib/server/proxy-headers.ts:1; src/lib/auth.ts:1; src/lib/source-script.ts:294; src/lib/safe-http.ts:1|S/O；Rust 中继有本机样本|原生最小凭据域、系统保管、脱敏诊断、IPC/loopback 能力撤销|保留并补齐服务端校验；管理员任意脚本是可信执行，不能当安全沙箱|AUTH-02、PLY-02、ADM-08|跨 origin/重定向/路径穿越/过期/越权拒绝；网络重试不重复危险写入|P0 / M / XL，完整审计未做|

## HEAD 缺失、已移除和仅宣称项（不充当当前实现）

|编号|候选能力与证据|核对结果|责任、依赖与验收|排期/状态|
|---|---|---|---|---|
|NSH-GAP-01|音乐/歌单/电子书：migrations/015_drop_music_and_books.sql:1；src/lib/config.ts:816|明确已整体移除，旧迁移名不能当现有音乐产品|如“原来的全部”包含历史删除能力，先确认历史版本/需求，再新增 NSH+服务端；验收须另补可追溯子矩阵|P2 条件范围 / U|
|NSH-GAP-02|私人影库 OpenList/Emby/小雅、追番订阅、网盘与求片：src/lib/config.ts:820；src/lib/config.ts:518|配置自检显式清理，含 NetDisk/Pansou、EnableMovieRequest，没有对应当前页面/API；收藏更新通知仍存在，不等于追番订阅|恢复需指定目标协议/历史版本，新增服务端适配和 NSH 页面；历史 changelog 不作为现状|P2 条件范围 / U|
|NSH-GAP-03|漫画/Suwayomi 遗留：src/lib/config.ts:518；migrations/006_manga.sql:1；migrations/postgres/006_manga.sql:1|有旧表迁移，但 src/app 当前没有漫画产品页面/API；不能由 SQL 表推断可用|若需恢复，另补来源/阅读器/同步合同；不在本轮擅自删表或执行迁移|P2 条件范围 / U|
|NSH-GAP-04|Netlify 与“离线体验”：README.md:90；README.md:45；next.config.js:1|部署宣称不等于当前可发布；PWA 缓存不等于媒体离线|运维侧复核实际托管配置/兼容性；离线媒体按 DL-03 验收|P2 条件 / U|

磁力缺口已以 NSH-SRC-08 纳入主表；没有找到 BT 下载服务或 BT 直接播放实现，不能把“资源链接”扩写成已实现 BT 播放。本轮不删现有网页、Android、KMP 或服务端。

## 覆盖与状态统计的解释

精确计数见路线图及机器可读 CSV。计数单位是稳定需求行，不是文件数或按钮数；一项的局部模拟通过不能让整模块标为完成。所有真实实例验收本轮均未执行。

后续验收台账应在同一编号下增加：服务端版本/SHA、存储类型、客户端版本/SHA、OS/DPI/设备、fixture 或真实数据来源、步骤、实际结果、日志/截图、失败与未测边界。不得覆盖现有历史证据。

