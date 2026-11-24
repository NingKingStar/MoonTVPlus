# 管理配置与个人设置逐字段补充表

基线 5b132342e7d36282e67edd73d188468d8c446ac6。这是主矩阵的细分检查清单，字段不另计为 130 项之外的新功能。每个叶字段的 NSH 交互、服务端责任、依赖、验收和阶段继承其父需求；不得用父项“有设置页”替代逐字段行为与保存回读验收。字段声明证明数据合同存在，不代表所有端都消费该字段。

## 管理合同

证据 src/lib/admin.types.ts；未收集配置实际值或任何密钥。对于 key/secret/password/token，原生只写/遮罩、服务器保管；否则按类型新写表单/枚举/数组编辑及校验。保留网页专用 CSS/统计脚本的原生管理编辑能力，但不在 NSH 裸执行。每项需验证“读取→改测试字段→保存→重读→功能生效”，且不覆盖未编辑字段。

|稳定细分键|类型声明 / 注释|父需求|证据|
|---|---|---|---|
|ConfigSubscribtion|{|NSH-ADM-05|src/lib/admin.types.ts:2|
|ConfigSubscribtion.URL|string;|NSH-ADM-05|src/lib/admin.types.ts:3|
|ConfigSubscribtion.AutoUpdate|boolean;|NSH-ADM-05|src/lib/admin.types.ts:4|
|ConfigSubscribtion.LastCheck|string;|NSH-ADM-05|src/lib/admin.types.ts:5|
|ConfigSubscribtion.Proxy|string;|NSH-ADM-05|src/lib/admin.types.ts:8|
|ConfigFile|string;|NSH-ADM-05|src/lib/admin.types.ts:10|
|SiteConfig|{|NSH-ADM-12/17/18|src/lib/admin.types.ts:11|
|SiteConfig.SiteName|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:12|
|SiteConfig.Announcement|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:13|
|SiteConfig.AnnouncementDisplayMode|'once' \| 'every';|NSH-ADM-12/17/18|src/lib/admin.types.ts:15|
|SiteConfig.SearchDownstreamMaxPage|number;|NSH-ADM-12/17/18|src/lib/admin.types.ts:16|
|SiteConfig.SiteInterfaceCacheTime|number;|NSH-ADM-12/17/18|src/lib/admin.types.ts:17|
|SiteConfig.DoubanProxyType|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:18|
|SiteConfig.DoubanProxy|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:19|
|SiteConfig.DoubanImageProxyType|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:20|
|SiteConfig.DoubanImageProxy|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:21|
|SiteConfig.DisableYellowFilter|boolean;|NSH-ADM-12/17/18|src/lib/admin.types.ts:22|
|SiteConfig.FluidSearch|boolean;|NSH-ADM-12/17/18|src/lib/admin.types.ts:23|
|SiteConfig.DanmakuSourceType|'builtin' \| 'custom';|NSH-ADM-12/17/18|src/lib/admin.types.ts:25|
|SiteConfig.DanmakuApiBase|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:26|
|SiteConfig.DanmakuApiToken|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:27|
|SiteConfig.DanmakuAutoLoadDefault|boolean; // 是否默认自动加载弹幕（用户可在本地覆盖）|NSH-ADM-12/17/18|src/lib/admin.types.ts:28|
|SiteConfig.TMDBApiKey|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:30|
|SiteConfig.TMDBProxy|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:31|
|SiteConfig.TMDBReverseProxy|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:32|
|SiteConfig.TMDBImageBaseUrl|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:34|
|SiteConfig.BangumiDataSource|'direct' / 'server-proxy' / 'custom-baseurl' / 'sakura';|NSH-ADM-12/17/18|src/lib/admin.types.ts:36|
|SiteConfig.BangumiApiBaseUrl|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:41|
|SiteConfig.BangumiImageBaseUrl|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:42|
|SiteConfig.BangumiProxy|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:43|
|SiteConfig.LiveChartProxy|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:44|
|SiteConfig.BannerDataSource|string; // 轮播图数据源：TMDB、TX 或 Douban|NSH-ADM-12/17/18|src/lib/admin.types.ts:45|
|SiteConfig.RecommendationDataSource|string; // 更多推荐数据源：Douban、TMDB、Mixed|NSH-ADM-12/17/18|src/lib/admin.types.ts:46|
|SiteConfig.LocalSettingsSyncMode|'off' \| 'manual' \| 'auto';|NSH-ADM-12/17/18|src/lib/admin.types.ts:48|
|SiteConfig.EnableComments|boolean;|NSH-ADM-12/17/18|src/lib/admin.types.ts:50|
|SiteConfig.CustomAdFilterCode|string;|NSH-ADM-12/17/18|src/lib/admin.types.ts:52|
|SiteConfig.CustomAdFilterVersion|number; // 代码版本号（时间戳）|NSH-ADM-12/17/18|src/lib/admin.types.ts:53|
|SiteConfig.EnableRegistration|boolean; // 开启注册|NSH-ADM-12/17/18|src/lib/admin.types.ts:55|
|SiteConfig.RequireRegistrationInviteCode|boolean; // 注册时要求邀请码|NSH-ADM-12/17/18|src/lib/admin.types.ts:56|
|SiteConfig.RegistrationInviteCode|string; // 通用注册邀请码|NSH-ADM-12/17/18|src/lib/admin.types.ts:57|
|SiteConfig.RegistrationRequireTurnstile|boolean; // 注册启用Cloudflare Turnstile|NSH-ADM-12/17/18|src/lib/admin.types.ts:58|
|SiteConfig.LoginRequireTurnstile|boolean; // 登录启用Cloudflare Turnstile|NSH-ADM-12/17/18|src/lib/admin.types.ts:59|
|SiteConfig.TurnstileSiteKey|string; // Cloudflare Turnstile Site Key|NSH-ADM-12/17/18|src/lib/admin.types.ts:60|
|SiteConfig.TurnstileSecretKey|string; // Cloudflare Turnstile Secret Key|NSH-ADM-12/17/18|src/lib/admin.types.ts:61|
|SiteConfig.DefaultUserTags|string[]; // 新注册用户的默认用户组|NSH-ADM-12/17/18|src/lib/admin.types.ts:62|
|SiteConfig.EnableOIDCLogin|boolean; // 启用OIDC登录|NSH-ADM-12/17/18|src/lib/admin.types.ts:64|
|SiteConfig.EnableOIDCRegistration|boolean; // 启用OIDC注册|NSH-ADM-12/17/18|src/lib/admin.types.ts:65|
|SiteConfig.OIDCIssuer|string; // OIDC Issuer URL (用于自动发现)|NSH-ADM-12/17/18|src/lib/admin.types.ts:66|
|SiteConfig.OIDCAuthorizationEndpoint|string; // 授权端点|NSH-ADM-12/17/18|src/lib/admin.types.ts:67|
|SiteConfig.OIDCTokenEndpoint|string; // Token端点|NSH-ADM-12/17/18|src/lib/admin.types.ts:68|
|SiteConfig.OIDCUserInfoEndpoint|string; // 用户信息端点|NSH-ADM-12/17/18|src/lib/admin.types.ts:69|
|SiteConfig.OIDCClientId|string; // OIDC Client ID|NSH-ADM-12/17/18|src/lib/admin.types.ts:70|
|SiteConfig.OIDCClientSecret|string; // OIDC Client Secret|NSH-ADM-12/17/18|src/lib/admin.types.ts:71|
|SiteConfig.OIDCButtonText|string; // OIDC登录按钮文字|NSH-ADM-12/17/18|src/lib/admin.types.ts:72|
|SiteConfig.OIDCMinTrustLevel|number; // 最低信任等级（仅LinuxDo网站有效，为0时不判断）|NSH-ADM-12/17/18|src/lib/admin.types.ts:73|
|SiteConfig.AnalyticsEnabled|boolean; // 是否启用流量统计|NSH-ADM-12/17/18|src/lib/admin.types.ts:75|
|SiteConfig.AnalyticsProvider|'umami' \| 'google' \| 'clarity' \| 'custom'; // 统计服务提供商|NSH-ADM-12/17/18|src/lib/admin.types.ts:76|
|SiteConfig.AnalyticsScriptUrl|string; // 脚本URL（Umami: umami.js地址; GA: gtag URL; 自定义: 脚本src）|NSH-ADM-12/17/18|src/lib/admin.types.ts:77|
|SiteConfig.AnalyticsWebsiteId|string; // 网站ID（Umami: website_id; GA: Measurement ID如G-XXXX; 自定义: 留空）|NSH-ADM-12/17/18|src/lib/admin.types.ts:78|
|SiteConfig.AnalyticsCustomScript|string; // 自定义统计代码（仅custom模式使用，完整的HTML脚本内容）|NSH-ADM-12/17/18|src/lib/admin.types.ts:79|
|UserConfig|{|NSH-ADM-02/03|src/lib/admin.types.ts:81|
|UserConfig.Users|{|NSH-ADM-02/03|src/lib/admin.types.ts:82|
|UserConfig.Users.username|string;|NSH-ADM-02/03|src/lib/admin.types.ts:83|
|UserConfig.Users.role|'user' \| 'admin' \| 'owner';|NSH-ADM-02/03|src/lib/admin.types.ts:84|
|UserConfig.Users.banned|boolean;|NSH-ADM-02/03|src/lib/admin.types.ts:85|
|UserConfig.Users.enabledApis|string[]; // 优先级高于tags限制|NSH-ADM-02/03|src/lib/admin.types.ts:86|
|UserConfig.Users.tags|string[]; // 多 tags 取并集限制|NSH-ADM-02/03|src/lib/admin.types.ts:87|
|UserConfig.Tags|{|NSH-ADM-02/03|src/lib/admin.types.ts:89|
|UserConfig.Tags.name|string;|NSH-ADM-02/03|src/lib/admin.types.ts:90|
|UserConfig.Tags.enabledApis|string[];|NSH-ADM-02/03|src/lib/admin.types.ts:91|
|UserConfig.Tags.permissions|string[];|NSH-ADM-02/03|src/lib/admin.types.ts:92|
|SpecialSourceApis|string[]; // 特殊源 key 列表，默认对普通入口隐藏|NSH-ADM-07|src/lib/admin.types.ts:95|
|ClientAdSourceApis|string[]; // 客户端去广告源 key 列表：MoonTVPlus APP / OrionTV 请求 source-detail 时 m3u8 套 proxy-m3u8|NSH-ADM-07|src/lib/admin.types.ts:96|
|SourceConfig|{|NSH-ADM-06/07|src/lib/admin.types.ts:97|
|SourceConfig.key|string;|NSH-ADM-06/07|src/lib/admin.types.ts:98|
|SourceConfig.name|string;|NSH-ADM-06/07|src/lib/admin.types.ts:99|
|SourceConfig.api|string;|NSH-ADM-06/07|src/lib/admin.types.ts:100|
|SourceConfig.detail|string;|NSH-ADM-06/07|src/lib/admin.types.ts:101|
|SourceConfig.from|'config' \| 'custom';|NSH-ADM-06/07|src/lib/admin.types.ts:102|
|SourceConfig.disabled|boolean;|NSH-ADM-06/07|src/lib/admin.types.ts:103|
|SourceConfig.proxyMode|boolean; // 代理模式开关：启用后由服务器代理m3u8和ts分片|NSH-ADM-06/07|src/lib/admin.types.ts:104|
|SourceConfig.weight|number; // 权重：用于排序和优选评分，默认0，范围0-100|NSH-ADM-06/07|src/lib/admin.types.ts:105|
|CustomCategories|{|NSH-ADM-09|src/lib/admin.types.ts:107|
|CustomCategories.name|string;|NSH-ADM-09|src/lib/admin.types.ts:108|
|CustomCategories.type|'movie' \| 'tv';|NSH-ADM-09|src/lib/admin.types.ts:109|
|CustomCategories.query|string;|NSH-ADM-09|src/lib/admin.types.ts:110|
|CustomCategories.from|'config' \| 'custom';|NSH-ADM-09|src/lib/admin.types.ts:111|
|CustomCategories.disabled|boolean;|NSH-ADM-09|src/lib/admin.types.ts:112|
|LiveRefreshIntervalHours|number; // 电视直播全局刷新间隔（小时），默认12小时|NSH-ADM-10|src/lib/admin.types.ts:114|
|LiveConfig|{|NSH-ADM-10|src/lib/admin.types.ts:115|
|LiveConfig.key|string;|NSH-ADM-10|src/lib/admin.types.ts:116|
|LiveConfig.name|string;|NSH-ADM-10|src/lib/admin.types.ts:117|
|LiveConfig.url|string; // m3u 地址|NSH-ADM-10|src/lib/admin.types.ts:118|
|LiveConfig.ua|string;|NSH-ADM-10|src/lib/admin.types.ts:119|
|LiveConfig.epg|string; // 节目单|NSH-ADM-10|src/lib/admin.types.ts:120|
|LiveConfig.from|'config' \| 'custom';|NSH-ADM-10|src/lib/admin.types.ts:121|
|LiveConfig.channelNumber|number;|NSH-ADM-10|src/lib/admin.types.ts:122|
|LiveConfig.disabled|boolean;|NSH-ADM-10|src/lib/admin.types.ts:123|
|LiveConfig.proxyMode|'full' \| 'm3u8-only' \| 'direct'; // 代理模式：full=全量代理，m3u8-only=仅代理m3u8，direct=直连|NSH-ADM-10|src/lib/admin.types.ts:124|
|WebLiveConfig|{|NSH-ADM-11|src/lib/admin.types.ts:126|
|WebLiveConfig.key|string;|NSH-ADM-11|src/lib/admin.types.ts:127|
|WebLiveConfig.name|string;|NSH-ADM-11|src/lib/admin.types.ts:128|
|WebLiveConfig.platform|string; // 直播平台类型，如 'huya'|NSH-ADM-11|src/lib/admin.types.ts:129|
|WebLiveConfig.roomId|string; // 房间ID|NSH-ADM-11|src/lib/admin.types.ts:130|
|WebLiveConfig.from|'config' \| 'custom';|NSH-ADM-11|src/lib/admin.types.ts:131|
|WebLiveConfig.disabled|boolean;|NSH-ADM-11|src/lib/admin.types.ts:132|
|WebLiveEnabled|boolean; // 网络直播功能总开关|NSH-ADM-11|src/lib/admin.types.ts:134|
|ThemeConfig|{|NSH-ADM-13|src/lib/admin.types.ts:135|
|ThemeConfig.enableBuiltInTheme|boolean; // 是否启用内置主题|NSH-ADM-13|src/lib/admin.types.ts:136|
|ThemeConfig.builtInTheme|string; // 内置主题名称|NSH-ADM-13|src/lib/admin.types.ts:137|
|ThemeConfig.customCSS|string; // 自定义CSS|NSH-ADM-13|src/lib/admin.types.ts:138|
|ThemeConfig.enableCache|boolean; // 是否启用浏览器缓存|NSH-ADM-13|src/lib/admin.types.ts:139|
|ThemeConfig.cacheMinutes|number; // 缓存时间（分钟）|NSH-ADM-13|src/lib/admin.types.ts:140|
|ThemeConfig.cacheVersion|number; // CSS版本号（用于缓存控制）|NSH-ADM-13|src/lib/admin.types.ts:141|
|ThemeConfig.loginBackgroundImage|string; // 登录界面背景图|NSH-ADM-13|src/lib/admin.types.ts:142|
|ThemeConfig.registerBackgroundImage|string; // 注册界面背景图|NSH-ADM-13|src/lib/admin.types.ts:143|
|ThemeConfig.homeBackgroundImage|string; // 首页背景图|NSH-ADM-13|src/lib/admin.types.ts:144|
|ThemeConfig.progressThumbType|'default' \| 'preset' \| 'custom'; // 图标类型|NSH-ADM-13|src/lib/admin.types.ts:146|
|ThemeConfig.progressThumbPresetId|string; // 预制图标ID|NSH-ADM-13|src/lib/admin.types.ts:147|
|ThemeConfig.progressThumbCustomUrl|string; // 自定义图标URL|NSH-ADM-13|src/lib/admin.types.ts:148|
|ThemeConfig.loadingStyle|'classic' \| 'grid' \| 'talisman';|NSH-ADM-13|src/lib/admin.types.ts:150|
|ThemeConfig.rateBadgeStyle|'default' \| 'flag' \| 'medal';|NSH-ADM-13|src/lib/admin.types.ts:152|
|AIConfig|{|NSH-ADM-14|src/lib/admin.types.ts:154|
|AIConfig.Enabled|boolean; // 是否启用AI问片功能|NSH-ADM-14|src/lib/admin.types.ts:155|
|AIConfig.Provider|'openai' \| 'claude' \| 'custom'; // AI服务提供商|NSH-ADM-14|src/lib/admin.types.ts:156|
|AIConfig.OpenAIApiKey|string;|NSH-ADM-14|src/lib/admin.types.ts:158|
|AIConfig.OpenAIBaseURL|string; // 自定义API地址（如Azure、国内代理等）|NSH-ADM-14|src/lib/admin.types.ts:159|
|AIConfig.OpenAIModel|string; // 模型名称，如gpt-4, gpt-3.5-turbo|NSH-ADM-14|src/lib/admin.types.ts:160|
|AIConfig.ClaudeApiKey|string;|NSH-ADM-14|src/lib/admin.types.ts:162|
|AIConfig.ClaudeBaseURL|string; // Claude Messages API根地址|NSH-ADM-14|src/lib/admin.types.ts:163|
|AIConfig.ClaudeModel|string; // 模型名称，如claude-3-opus-20240229|NSH-ADM-14|src/lib/admin.types.ts:164|
|AIConfig.CustomApiKey|string;|NSH-ADM-14|src/lib/admin.types.ts:166|
|AIConfig.CustomBaseURL|string;|NSH-ADM-14|src/lib/admin.types.ts:167|
|AIConfig.CustomModel|string;|NSH-ADM-14|src/lib/admin.types.ts:168|
|AIConfig.EnableDecisionModel|boolean; // 是否启用决策模型（用AI判断是否需要联网/数据源）|NSH-ADM-14|src/lib/admin.types.ts:170|
|AIConfig.DecisionProvider|'openai' \| 'claude' \| 'custom'; // 决策模型提供商|NSH-ADM-14|src/lib/admin.types.ts:171|
|AIConfig.DecisionOpenAIApiKey|string;|NSH-ADM-14|src/lib/admin.types.ts:172|
|AIConfig.DecisionOpenAIBaseURL|string;|NSH-ADM-14|src/lib/admin.types.ts:173|
|AIConfig.DecisionOpenAIModel|string;|NSH-ADM-14|src/lib/admin.types.ts:174|
|AIConfig.DecisionClaudeApiKey|string;|NSH-ADM-14|src/lib/admin.types.ts:175|
|AIConfig.DecisionClaudeModel|string;|NSH-ADM-14|src/lib/admin.types.ts:176|
|AIConfig.DecisionCustomApiKey|string;|NSH-ADM-14|src/lib/admin.types.ts:177|
|AIConfig.DecisionCustomBaseURL|string;|NSH-ADM-14|src/lib/admin.types.ts:178|
|AIConfig.DecisionCustomModel|string;|NSH-ADM-14|src/lib/admin.types.ts:179|
|AIConfig.EnableWebSearch|boolean; // 是否启用联网搜索|NSH-ADM-14|src/lib/admin.types.ts:181|
|AIConfig.WebSearchProvider|'tavily' \| 'serper' \| 'serpapi' \| 'bing'; // 搜索服务提供商|NSH-ADM-14|src/lib/admin.types.ts:182|
|AIConfig.TavilyApiKey|string; // Tavily API密钥|NSH-ADM-14|src/lib/admin.types.ts:183|
|AIConfig.SerperApiKey|string; // Serper.dev API密钥|NSH-ADM-14|src/lib/admin.types.ts:184|
|AIConfig.SerpApiKey|string; // SerpAPI密钥|NSH-ADM-14|src/lib/admin.types.ts:185|
|AIConfig.EnableNewMode|boolean; // 是否启用新版工具式调用（LLM 工具/function-calling），默认 true|NSH-ADM-14|src/lib/admin.types.ts:187|
|AIConfig.NewProtocol|'openai-completions' \| 'openai-responses' \| 'claude'; // 新版协议，默认 openai-completions|NSH-ADM-14|src/lib/admin.types.ts:188|
|AIConfig.MaxContext|number; // 最大上下文 token 数，默认 131072（128k）|NSH-ADM-14|src/lib/admin.types.ts:189|
|AIConfig.CompressThreshold|number; // 上下文压缩触发阈值百分比（0-100），默认 90；0=关闭|NSH-ADM-14|src/lib/admin.types.ts:190|
|AIConfig.EnableHomepageEntry|boolean; // 首页入口开关|NSH-ADM-14|src/lib/admin.types.ts:192|
|AIConfig.EnableVideoCardEntry|boolean; // VideoCard入口开关|NSH-ADM-14|src/lib/admin.types.ts:193|
|AIConfig.EnablePlayPageEntry|boolean; // 播放页入口开关|NSH-ADM-14|src/lib/admin.types.ts:194|
|AIConfig.EnableAIComments|boolean; // AI评论生成开关|NSH-ADM-14|src/lib/admin.types.ts:195|
|AIConfig.EnableAICommentsToolMode|boolean; // AI评论走工具式调用（模型自主联网/查豆瓣/TMDB），默认关闭|NSH-ADM-14|src/lib/admin.types.ts:196|
|AIConfig.Temperature|number; // AI温度参数（0-2），默认0.7|NSH-ADM-14|src/lib/admin.types.ts:198|
|AIConfig.MaxTokens|number; // 最大回复token数，默认1000|NSH-ADM-14|src/lib/admin.types.ts:199|
|AIConfig.SystemPrompt|string; // 自定义系统提示词|NSH-ADM-14|src/lib/admin.types.ts:200|
|AIConfig.EnableStreaming|boolean; // 是否启用流式响应，默认true|NSH-ADM-14|src/lib/admin.types.ts:201|
|AIConfig.DefaultMessageNoVideo|string; // 无视频时的默认消息|NSH-ADM-14|src/lib/admin.types.ts:203|
|AIConfig.DefaultMessageWithVideo|string; // 有视频时的默认消息（支持 {title} 替换符）|NSH-ADM-14|src/lib/admin.types.ts:204|
|EmailConfig|{|NSH-ADM-15|src/lib/admin.types.ts:206|
|EmailConfig.enabled|boolean; // 是否启用邮件通知|NSH-ADM-15|src/lib/admin.types.ts:207|
|EmailConfig.provider|'smtp' \| 'resend'; // 邮件发送方式|NSH-ADM-15|src/lib/admin.types.ts:208|
|EmailConfig.smtp|{|NSH-ADM-15|src/lib/admin.types.ts:210|
|EmailConfig.smtp.host|string; // SMTP服务器地址|NSH-ADM-15|src/lib/admin.types.ts:211|
|EmailConfig.smtp.port|number; // SMTP端口（25/465/587）|NSH-ADM-15|src/lib/admin.types.ts:212|
|EmailConfig.smtp.secure|boolean; // 是否使用SSL/TLS|NSH-ADM-15|src/lib/admin.types.ts:213|
|EmailConfig.smtp.user|string; // SMTP用户名|NSH-ADM-15|src/lib/admin.types.ts:214|
|EmailConfig.smtp.password|string; // SMTP密码|NSH-ADM-15|src/lib/admin.types.ts:215|
|EmailConfig.smtp.from|string; // 发件人邮箱|NSH-ADM-15|src/lib/admin.types.ts:216|
|EmailConfig.resend|{|NSH-ADM-15|src/lib/admin.types.ts:219|
|EmailConfig.resend.apiKey|string; // Resend API Key|NSH-ADM-15|src/lib/admin.types.ts:220|
|EmailConfig.resend.from|string; // 发件人邮箱|NSH-ADM-15|src/lib/admin.types.ts:221|
|TelegramConfig|{|NSH-ADM-16|src/lib/admin.types.ts:224|
|TelegramConfig.enabled|boolean; // 是否启用 Telegram Bot|NSH-ADM-16|src/lib/admin.types.ts:225|
|TelegramConfig.botToken|string; // Bot Token，仅服务端使用|NSH-ADM-16|src/lib/admin.types.ts:226|
|TelegramConfig.botUsername|string; // Bot 用户名，用于前端跳转|NSH-ADM-16|src/lib/admin.types.ts:227|
|TelegramConfig.webhookSecret|string; // Webhook Secret Token|NSH-ADM-16|src/lib/admin.types.ts:228|
|TelegramConfig.apiProxy|string; // Telegram Bot API 系统代理（HTTP/HTTPS proxy）|NSH-ADM-16|src/lib/admin.types.ts:229|
|TelegramConfig.apiBaseUrl|string; // Telegram Bot API 反代 Base URL|NSH-ADM-16|src/lib/admin.types.ts:230|
|TelegramConfig.loginEnabled|boolean; // 是否启用 Telegram 登录|NSH-ADM-16|src/lib/admin.types.ts:231|
|TelegramConfig.bindingEnabled|boolean; // 是否启用用户绑定|NSH-ADM-16|src/lib/admin.types.ts:232|
|TelegramConfig.registrationEnabled|boolean; // 是否启用 Telegram 注册|NSH-ADM-16|src/lib/admin.types.ts:233|
|TelegramConfig.notificationsEnabled|boolean; // 是否启用 Telegram 通知|NSH-ADM-16|src/lib/admin.types.ts:234|
|TelegramConfig.defaultNotifications|boolean; // 新绑定用户默认开启通知|NSH-ADM-16|src/lib/admin.types.ts:235|

## 个人设置云同步白名单

逐项父需求为 NSH-CFG-01/03，并关联相应播放/搜索/下载能力。结构 version=1，data 为字符串值；原生应按类型解析并限白名单。filesystemSavePath 在旧端虽列白名单，原生本机绝对路径/访问授权不得照搬上传；需使用可迁移偏好与本机路径分离的兼容映射。

|键（稳定细分键 CFG-03.键名）|证据|原生验收要求|
|---|---|---|
|defaultAggregateSearch|src/lib/local-settings-sync.ts:14|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|saveLivePlayRecords|src/lib/local-settings-sync.ts:15|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|liveAutoSpeedTest|src/lib/local-settings-sync.ts:16|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|enableOptimization|src/lib/local-settings-sync.ts:17|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|preferStrategy|src/lib/local-settings-sync.ts:18|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|preferMode|src/lib/local-settings-sync.ts:19|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|speedTestTimeout|src/lib/local-settings-sync.ts:20|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|maxConcurrentDownloads|src/lib/local-settings-sync.ts:21|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|downloadThreadsPerTask|src/lib/local-settings-sync.ts:22|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|downloadSegmentTimeout|src/lib/local-settings-sync.ts:23|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|downloadMode|src/lib/local-settings-sync.ts:24|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|filesystemSavePath|src/lib/local-settings-sync.ts:25|本机目录单独保留，跨端不同路径需重新授权，不照搬远端绝对路径|
|fluidSearch|src/lib/local-settings-sync.ts:26|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|tmdb_backdrop_disabled|src/lib/local-settings-sync.ts:27|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|doubanProxyUrl|src/lib/local-settings-sync.ts:28|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|doubanDataSource|src/lib/local-settings-sync.ts:29|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|doubanDataSourceBackup|src/lib/local-settings-sync.ts:30|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|doubanProxyUrlBackup|src/lib/local-settings-sync.ts:31|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|animeDataSource|src/lib/local-settings-sync.ts:32|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|animeDataSourceBackup|src/lib/local-settings-sync.ts:33|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|animeCustomBaseUrl|src/lib/local-settings-sync.ts:34|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|animeImageBaseUrl|src/lib/local-settings-sync.ts:35|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|doubanImageProxyType|src/lib/local-settings-sync.ts:36|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|doubanImageProxyUrl|src/lib/local-settings-sync.ts:37|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|doubanImageProxyTypeBackup|src/lib/local-settings-sync.ts:38|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|doubanImageProxyUrlBackup|src/lib/local-settings-sync.ts:39|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|tmdbImageBaseUrl|src/lib/local-settings-sync.ts:40|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|bufferStrategy|src/lib/local-settings-sync.ts:41|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|nextEpisodePreCache|src/lib/local-settings-sync.ts:42|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|nextEpisodeDanmakuPreload|src/lib/local-settings-sync.ts:43|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|disablePlaybackThumbnail|src/lib/local-settings-sync.ts:44|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|disableDuanjuVertical|src/lib/local-settings-sync.ts:45|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|disableEpisodeTitleFetch|src/lib/local-settings-sync.ts:46|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|disableAutoLoadDanmaku|src/lib/local-settings-sync.ts:47|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|danmakuMaxCount|src/lib/local-settings-sync.ts:48|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|danmaku_heatmap_disabled|src/lib/local-settings-sync.ts:49|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|homeContinueWatchingEnabled|src/lib/local-settings-sync.ts:50|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|homeModules|src/lib/local-settings-sync.ts:51|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|danmakuTraditionalToSimplified|src/lib/local-settings-sync.ts:52|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|searchTraditionalToSimplified|src/lib/local-settings-sync.ts:53|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|exactSearch|src/lib/local-settings-sync.ts:54|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|quickForwardSeconds|src/lib/local-settings-sync.ts:56|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|
|seekStepSeconds|src/lib/local-settings-sync.ts:57|按现有语义恢复/生效；缺失键用默认；坏值不使界面失效|

管理合同字段声明 189 项，个人设置白名单 43 键。个人通知/设备/TVBOX/Telegram 等账户操作不在此白名单，另按 AUTH/NTF/TG/TV 主矩阵验收。
