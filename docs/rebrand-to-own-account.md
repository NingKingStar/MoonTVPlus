# 把 MoonTVPlus 换成自己的账号与自部署身份

- 日期：2026-11-27（Asia/Shanghai）
- 基线：`36f3bc29`（fork 自 `Happier-X/MoonTVPlus`，origin 为 `NingKingStar/MoonTVPlus`）
- 范围：仓库里「对外可见的身份」——README、UI 链接、镜像名、发布/部署回退地址、定时任务口令。
- 不在范围：`apps/desktop-nsh/`（另一个任务在改）；`package.json` 与 `.github/workflows/verify-notes.yml` 的改动来自别处，未触碰。

## 1. 盘点结果：哪些标识属于谁

全仓检索的标识与分类如下。分类口径：

- **A 本项目自身身份**：换成 `NingKingStar/MoonTVPlus` 之后仍然真实存在（仓库、镜像、发布产物）。
- **B 前账号的个人基础设施**：域名、口令这类「属于某个人」的东西，改为可配置或无默认。
- **C 独立第三方项目**：别人的仓库/构建/服务，保持原样以免 404，只在文案上标明「上游第三方」。

| 标识 | 出现位置 | 分类 | 处理 |
| --- | --- | --- | --- |
| `monvpls` | 全仓 0 处（只有正确的 `mtvpls` 命名空间） | — | 无需修改 |
| `Happier-X/MoonTVPlus` | `cloudflare-deploy.yml`、`cloudflare-worker.ts`、`version_check.ts` | A（发布回退仓库 / 版本检查源） | 改为本仓库 |
| `tv.370125.xyz` | `cloudflare-deploy.yml`（`latest.json` 下载地址） | B（个人域名） | 改为 `SITE_BASE` 或本仓库 Release 直链 |
| `ghcr.io/mtvpls/moontvplus*` | `docker-image.yml`、`docker-image-lite.yml`、`README.md`、`render.yaml` | A（本项目镜像名） | 改为 `ghcr.io/<仓库所有者小写>/moontvplus` |
| `mtvpls` 作为 `CRON_PASSWORD` 默认值 | `README.md`、`render.yaml`、`start.js`、`cloudflare-worker.ts`、`api/cron/[password]/route.ts`、`vercel.json` | B（写死的口令） | 去掉默认值，缺失即拒绝/停用 |
| `mtvpls-danmu.netlify.app` | `src/lib/config.ts`、`src/lib/danmaku/config.ts`、`src/app/admin/page.tsx` | B（个人部署的弹幕服务） | 改为环境变量提供，缺失时回退自定义源 |
| `star-history.dera.page` | `README.md` | A（只是第三方图床，仓库名是参数） | 仅把 `repos=` 参数换成本仓库 |
| `mtvpls/MoonTVPlus-PC` | `src/components/UserMenu.tsx` | C（前作者维护的独立客户端仓库） | 保留链接，文案标注「上游第三方构建」 |
| `mtvpls/Selene-Build` | `src/components/UserMenu.tsx` | C | 同上 |
| `mtvpls/OrionTV_Build` | `src/components/UserMenu.tsx`（2 处） | C | 同上 |
| `mtvpls/moontvplus-extension` | `src/components/UserMenu.tsx`、`src/app/layout.tsx`（指纹标记） | C（浏览器扩展；`layout.tsx` 的标记是扩展识别本站的接口） | 链接保留并标注；指纹标记不动 |
| `MoonTVPlus-PC` / `Selene` / `OrionTV` 文案 | `UserMenu.tsx`、`apps/**` 注释 | C | 文案加「上游第三方构建，非本仓库发布」 |
| `Copyright © ymyuuu` | `proxy.worker.js`（第三方 Proxy Everything 脚本） | C | 保留，删掉反而抹掉别人署名 |

## 2. 逐处改动清单（文件:行 → 旧值 → 新值 → 理由）

### 2.1 README.md

| 行 | 旧值 | 新值 | 理由 |
| --- | --- | --- | --- |
| 94 | Vercel 一键部署 `repository-url=https://github.com/mtvpls/MoonTVPlus` | `https://github.com/NingKingStar/MoonTVPlus` | 部署按钮必须指向本仓库，否则点下去复制的是前账号的仓库 |
| 96 | Netlify `deploy?repository=.../mtvpls/MoonTVPlus` | `.../NingKingStar/MoonTVPlus` | 同上 |
| 104 | Render `deploy?repo=.../mtvpls/MoonTVPlus` | `.../NingKingStar/MoonTVPlus` | 同上 |
| 274-278 | （无） | 新增「镜像来源」提示：首次需先手动跑一次 `Build & Push Docker image`（Lite 另跑一次），否则 `docker pull` 会 404 | 镜像名换成自己账号后，GHCR 里还没有任何标签，必须提前告知 |
| 284/319/339/376 | `image: ghcr.io/mtvpls/moontvplus:latest` | `ghcr.io/ningkingstar/moontvplus:latest` | 镜像归属改为本账号；GHCR 路径必须全小写 |
| 391/398 | `ghcr.io/mtvpls/moontvplus-lite:latest` | `ghcr.io/ningkingstar/moontvplus-lite:latest` | 同上 |
| 489 | `CRON_PASSWORD` 默认值 `mtvpls` | 「无默认，必填」，并说明未设置时 `/api/cron` 返回 503、内置定时任务停用，Vercel 的 `crons.path` 必须与口令一致 | 写死的口令等于公开口令 |
| 534 | `DANMAKU_API_TOKEN` 默认 `87654321` | `(空)` | 不再默认使用前账号那套弹幕服务的令牌 |
| 535-536 | （无） | 新增 `BUILTIN_DANMAKU_API_BASE` / `BUILTIN_DANMAKU_API_TOKEN` 两个环境变量 | 后台「内置源」改为部署方自己提供 |
| 689-697 | （无） | 新增「上游与许可」小节（并加入目录）：上游仓库、更早血统、`LICENSE` 为 CC BY-NC-SA 4.0 且原样保留、本仓库不代表上游、「生态应用」属上游第三方构建 | CC BY-NC-SA 要求署名；同时把「哪些是别人的东西」讲清楚，避免被误认为本仓库发布 |
| 711 | `star-history.dera.page/svg?repos=mtvpls/moontvplus` | `repos=NingKingStar/MoonTVPlus`（含锚点参数） | 图表参数指向本仓库；图床本身是第三方服务，不改 |

### 2.2 UI 链接与版本检查

| 文件:行 | 旧值 | 新值 | 理由 |
| --- | --- | --- | --- |
| `src/components/VersionPanel.tsx:164` | `raw.githubusercontent.com/mtvpls/MoonTVPlus/main/CHANGELOG` | `raw.githubusercontent.com/NingKingStar/MoonTVPlus/main/CHANGELOG` | 版本面板拉的是本仓库 CHANGELOG，换成前账号地址会读到别人的更新说明 |
| `src/components/VersionPanel.tsx:461,491` | `https://github.com/mtvpls/MoonTVPlus.git` | `https://github.com/NingKingStar/MoonTVPlus` | 「前往仓库」按钮；顺手去掉无意义的 `.git` 后缀 |
| `src/app/login/page.tsx:38` | `https://github.com/mtvpls/MoonTVPlus` | `https://github.com/NingKingStar/MoonTVPlus` | 登录页外链 |
| `src/app/register/page.tsx:38` | 同上 | 同上 | 注册页外链 |
| `src/lib/version_check.ts:17,19` | `raw.githubusercontent.com/Happier-X/...`、`cdn.jsdelivr.net/gh/Happier-X/...` | `NingKingStar/...` | 注释写着「只和本仓库 VERSION.txt 比较」，地址却还是上游；fork 里 `VERSION.txt` 存在，不会 404 |

### 2.3 容器镜像与发布工作流

| 文件:行 | 旧值 | 新值 | 理由 |
| --- | --- | --- | --- |
| `.github/workflows/docker-image.yml:64` | `images: ghcr.io/mtvpls/moontvplus` | `ghcr.io/${{ steps.lowercase.outputs.owner }}/moontvplus` | 镜像归属跟随仓库所有者；仓库里本来就有 `Set lowercase repository owner` 步骤（一直是死代码），这里终于用上，避免 `NingKingStar` 大写导致 Docker 引用非法 |
| `docker-image.yml:78-80` | 两条 tag + `name=ghcr.io/mtvpls/...` | 同上写法 | 同上 |
| `docker-image.yml:133-135` | manifest 合并的 3 处 `ghcr.io/mtvpls/...` | 同上 | 同上 |
| `docker-image.yml:146` | `owner: mtvpls`（ghcr 清理 action） | `owner: ${{ github.repository_owner }}` | 清理目标跟着仓库走，否则清的是别人的包 |
| `.github/workflows/docker-image-lite.yml:45-48,114-117` | （无此步骤） | 两个 job 各加 `Set lowercase repository owner` | Lite 工作流原本没有这个步骤，但镜像名已改为引用它 |
| `docker-image-lite.yml:63,77-79,132-134` | `ghcr.io/mtvpls/moontvplus-lite` | `ghcr.io/${{ steps.lowercase.outputs.owner }}/moontvplus-lite` | 同上 |
| `docker-image-lite.yml:145` | `owner: mtvpls` | `owner: ${{ github.repository_owner }}` | 同上 |
| `.github/workflows/docker-image-dev.yml:49-60,142-148` | `IMAGE_NAME=ghcr.io/${{ github.repository_owner }}/...` 直接用原始所有者 | 先 `OWNER=$(... | tr '[:upper:]' '[:lower:]')` 再拼 | 原写法在 `NingKingStar` 这种带大写的账号下会生成非法镜像名；三处 job 一致处理 |

### 2.4 发布回退与安卓更新包（Cloudflare 部署）

| 文件:行 | 旧值 | 新值 | 理由 |
| --- | --- | --- | --- |
| `cloudflare-deploy.yml:194-195` | （无） | 新增 `SITE_BASE: ${{ vars.SITE_BASE }}` | 站点域名由仓库变量提供，缺省走下面的兜底 |
| `cloudflare-deploy.yml:199` | `gh release list -R Happier-X/MoonTVPlus` | `-R ${{ github.repository }}` | 站点从「自己的仓库」找 client 发布，不再依赖前账号仓库 |
| `cloudflare-deploy.yml:207` | `gh release view ... -R Happier-X/MoonTVPlus` | `-R ${{ github.repository }}` | 同上 |
| `cloudflare-deploy.yml:213` | `gh api "repos/Happier-X/MoonTVPlus/releases/tags/$TAG"` | `repos/${{ github.repository }}/...` | 同上 |
| `cloudflare-deploy.yml:220` | `gh release view ... -R Happier-X/MoonTVPlus` | `-R ${{ github.repository }}` | 同上 |
| `cloudflare-deploy.yml:230-236` | `--arg url "https://tv.370125.xyz/downloads/moontvplus-phone-latest.apk"` | 有 `SITE_BASE` 时 `"$SITE_BASE/downloads/moontvplus-phone-latest.apk"`，否则 `https://github.com/${{ github.repository }}/releases/download/$TAG/$ASSET` | 去掉前账号域名；两种取值都真实存在，不会 404 |
| `cloudflare-worker.ts:47-49` | `github.com/Happier-X/MoonTVPlus/releases/download/client-v...` | `github.com/NingKingStar/MoonTVPlus/...`，并加注释说明「仓库改名只需改这一处」 | Worker 直出 APK 时的上游地址 |

> `cloudflare-deploy.yml` 里 `paths-ignore`、`environment: server-production`、删掉 `cat wrangler.toml` 这三处是别的任务已经改好的 CI 隔离改动，本次未动。

### 2.5 定时任务口令：从「有默认值」变成「缺失即停用」

| 文件:行 | 旧值 | 新值 | 理由 |
| --- | --- | --- | --- |
| `src/app/api/cron/[password]/route.ts:34-46` | `process.env.CRON_PASSWORD \|\| 'mtvpls'` | 未设置 → 打一条告警并返回 `503 {message: 'Cron disabled: CRON_PASSWORD is not set'}` | 原来任何人都能用公开的 `mtvpls` 触发收藏更新/直播刷新；现在默认关闭 |
| `start.js:26-30` | — | 模块级 `const cronPassword = (process.env.CRON_PASSWORD \|\| '').trim()` | 单一口径 |
| `start.js:47-52` | 服务起来后无条件排定时任务 | 无口令时打印停用原因并 `return`，不再排定时器 | 避免每小时用空口令打 401 |
| `start.js:74-76` | 函数内再读一次 `\|\| 'mtvpls'` | 复用模块级变量，空则直接返回 | 同上 |
| `start.js:80-81` | `console.log(\`Executing cron job: ${cronUrl}\`)` | `console.log('Executing internal cron job ...')` | 原日志把口令打进 stdout |
| `cloudflare-worker.ts:82-89` | `env.CRON_PASSWORD \|\| 'mtvpls'` | 空则 `console.warn` 并 `return` | Worker 的 scheduled 同样不能靠猜的口令 |
| `render.yaml:16-18` | `CRON_PASSWORD: value: mtvpls` | `generateValue: true` | 让 Render 生成随机值；想自己指定就改 `sync: false` |
| `vercel.json:15` | `"path": "/api/cron/mtvpls"` | `"path": "/api/cron/CHANGE_ME_CRON_PASSWORD"` | Vercel Cron 只能打静态路径，路径必须等于口令；占位串是明确的「请自己改」，未改则等于未配置 |

**行为变化**：升级到本版本后，若环境里没有 `CRON_PASSWORD`，`/api/cron/*` 会返回 503、容器内的每小时任务不再启动。这是有意的失败关闭（fail-closed）；设置一个随机口令即可恢复。

### 2.6 内置弹幕源：不再内置别人的部署

| 文件:行 | 旧值 | 新值 | 理由 |
| --- | --- | --- | --- |
| `src/lib/danmaku/config.ts:3-6` | 常量写死 `https://mtvpls-danmu.netlify.app/...` 与固定令牌 | `BUILTIN_DANMAKU_API_BASE` / `BUILTIN_DANMAKU_API_TOKEN` 读同名环境变量，默认空串 | 个人 Netlify 域名出现在前端请求和后台默认值里 |
| `config.ts:13-14` | `if (DanmakuSourceType === 'builtin') return BUILTIN...` | `builtin && BUILTIN...` 才走内置，否则落到自定义地址逻辑 | 未配置时「内置源」自动退化为自定义源，不会请求空地址 |
| `config.ts:20-22` | `token === BUILTIN_TOKEN ? base : base/token` | `token && token !== BUILTIN_TOKEN ? base/token : base` | 令牌为空时不再拼出 `base/` |
| `src/lib/config.ts:10-15` | 同一个写死的 URL | 两个 `process.env.BUILTIN_DANMAKU_API_*` 常量 | 服务端默认配置同样去个人化 |
| `src/lib/config.ts:320-328` | `DanmakuSourceType: hasCustomDanmakuEnv ? 'custom' : 'builtin'` 等 | 加 `\|\| !BUILTIN_DANMAKU_API_BASE` 条件，未配置内置服务时默认 `custom` + `http://localhost:9321` | 没有内置服务却显示「内置源」会误导 |
| `src/lib/config.ts:561-563,583-589` | 默认值/回填值使用写死的 URL 与令牌 | 用环境变量常量，空则回退 `localhost:9321` | 同上 |
| `src/app/admin/page.tsx:7942-7943` | 后台初始值写死个人 URL 与令牌 | `DanmakuApiBase: ''`、`DanmakuApiToken: ''` | 后台界面的默认值同样是「对外身份」 |
| `src/app/admin/page.tsx:8741` | 「内置弹幕源为多人共享服务…」 | 说明「内置源由部署方提供，需设置 `BUILTIN_DANMAKU_API_BASE`（可选 `_TOKEN`），未设置时按自定义源请求」 | 让站长知道为什么「内置源」可能不生效 |

### 2.7 第三方客户端标注

| 文件:行 | 旧值 | 新值 | 理由 |
| --- | --- | --- | --- |
| `UserMenu.tsx:5879` | 「专为 Windows 开发的客户端…」 | 末尾加「（上游第三方构建，非本仓库发布）」 | 链接必须保留（换成自己的仓库会 404），但要说明不是本仓库的产物 |
| `UserMenu.tsx:5913` | 「多平台客户端」 | 同上标注 | Selene 是上游的二次开发构建 |
| `UserMenu.tsx:5949` | 「tv专用」 | 同上标注 | OrionTV 同上 |
| `UserMenu.tsx:5981` | 「为 MoonTVPlus / 为直链播放提供…」（重复了一行） | 合并成一句并加同上标注 | 顺手修掉重复文案 |

## 3. 你必须自己去做的事

### GitHub（`NingKingStar/MoonTVPlus`）

1. **首次构建镜像**：Actions → `Build & Push Docker image`（需要 Lite 就再跑 `Build & Push Docker lite image`）→ Run workflow。跑完才会有 `ghcr.io/ningkingstar/moontvplus(:latest)`，README 里已写明这一点。
2. **GHCR 包可见性**：新包默认 private。要公开拉取，去 `https://github.com/users/NingKingStar/packages/container/moontvplus/settings` 把 visibility 改成 public，并确认包已关联到本仓库（Repository → Package settings → Connect repository）。
3. **Actions 权限**：Settings → Actions → General → Workflow permissions 至少允许 `Read and write permissions`，否则 `GITHUB_TOKEN` 推不了 GHCR、也建不了 Release。
4. **仓库变量 `SITE_BASE`**（可选）：Settings → Secrets and variables → Actions → Variables → 新建 `SITE_BASE = https://你的域名`。不设也能跑，此时 `latest.json` 的 APK 地址指向本仓库 Release 直链。
5. **Secrets**：`CRON_PASSWORD`（随机长串）、以及原有的云平台相关 secrets。**不设 `CRON_PASSWORD` 定时任务就不工作**，这是有意的。

### Cloudflare

1. Workers/Pages 的 `CRON_PASSWORD` secret 要和 GitHub 里的一致（`cloudflare-deploy.yml` 会用 `secrets.CRON_PASSWORD` 写进 `wrangler.toml` 的 `[vars]`）。
2. `SITE_BASE`：工作流里既读 `vars.SITE_BASE`（第 195 行）也读 `secrets.SITE_BASE`（第 72/144 行用于构建期站点地址），建议两处写同一个值或干脆都不写（走 Release 直链）。

### Vercel / Render

1. Vercel：`CRON_PASSWORD` 必须等于 `vercel.json` 里 `crons.path` 的最后一段。仓库里写的是 `CHANGE_ME_CRON_PASSWORD`，请改成你自己的口令（口令同时会进 git，介意的话就别在 Vercel 上用 Cron，改用外部定时器打 `/api/cron/<口令>`）。
2. Render：`render.yaml` 已改成 `generateValue: true`（Render 生成随机值）。如果你想让 GitHub 里的值和 Render 里的一致，改成 `sync: false` 并在控制台手动填。

## 4. 故意没改的地方（以及为什么）

| 位置 | 内容 | 原因 |
| --- | --- | --- |
| `UserMenu.tsx:5535,5882,5917,5952,5984` | `github.com/mtvpls/{MoonTVPlus-PC,Selene-Build,OrionTV_Build,moontvplus-extension}` | 这些仓库确实存在且由上游作者发布；改成 `NingKingStar/...` 只会得到 404。文案已标注「上游第三方构建」 |
| `src/app/layout.tsx:42,323` | `moontvplus-extension` 识别指纹 | 这是浏览器扩展识别本站的约定标记，改了扩展就认不出本站 |
| `LICENSE` | CC BY-NC-SA 4.0 全文 | 许可要求保留署名；本仓库未删改 |
| `README.md:688` | `[MIT](LICENSE) © 2025 MoonTV & Contributors` | 原作者的版权声明行，按要求保留。注意它与 `LICENSE` 文件（CC BY-NC-SA 4.0）不一致，这是上游历史遗留；要改授权只能由版权所有者决定，已在「上游与许可」里写明 |
| `CHANGELOG`、git 历史 | 上游提交记录 | 历史就是历史，改写反而破坏可追溯性 |
| `proxy.worker.js:168-169` | `Copyright © ymyuuu` | 第三方 Proxy Everything 脚本自带署名，删掉是侵权 |
| `docs/nsh-*.md`、`docs/nsh-*.csv/json` | 大量 `Happier-X/MoonTVPlus` 链接 | 另一个任务的基线审计文档，链接带 commit SHA，属于「历史证据」而不是产品身份；由该任务负责 |
| `apps/android-phone/**`、`apps/nsh-phone/**` | `UpdatePolicy.REPOSITORY = "Happier-X/MoonTVPlus"`、`SITE_BASE_URL/SITE_APK_URL = https://tv.370125.xyz`、`DEFAULT_SERVER = "https://tv.370125.xyz"` | **需要你先给一个域名**（见第 6 节），否则我只能凭空编一个地址，安卓端的默认服务器会直接不可用 |
| `package.json` | `notes:*` 脚本等改动 | 不是本次任务的改动，未触碰 |

## 5. 验证

| 项目 | 结果 |
| --- | --- |
| `npx tsc --noEmit --incremental false`（= `pnpm typecheck`） | 通过，0 错误 |
| `next lint`（= `pnpm lint`） | 仓库级仍有 **51 个既有错误**（`play/page.tsx`、`watch-room/*`、若干 api route 等，均不在本次改动范围内）。本次改动涉及的 `src/lib/config.ts`、`src/lib/danmaku/config.ts`、`src/lib/version_check.ts`、`src/app/api/cron/[password]/route.ts`、`src/app/login/page.tsx`、`src/app/register/page.tsx` 单独跑 eslint 均为 **0 error**（`UserMenu.tsx`、`admin/page.tsx` 也是 0 error，只剩既有的 `<img>` 警告） |
| `prettier -c`（= `pnpm format:check`） | **基线就不过**：把 HEAD 版本单独拿出来跑也报同样的文件（仓库现有代码本身不符合 `.prettierrc.js` 的 80 列），所以它不能当门槛。本次新增/修改的行按 prettier 输出对齐了；`render.yaml`、`vercel.json` 通过检查 |
| YAML 可解析 | `docker-image.yml`、`docker-image-lite.yml`、`docker-image-dev.yml`、`cloudflare-deploy.yml`、`render.yaml` 用 prettier 的 YAML parser 解析全部 OK（无 SyntaxError） |
| JSON 可解析 | `vercel.json`、`package.json` 通过 `JSON.parse` |

未执行（按要求）：不跑 docker build、不触发任何 workflow、不 commit/push。

## 6. 未完成 / 需要你决定的

1. **安卓与 NSH 客户端的默认服务器**（`apps/android-phone/app/src/main/java/com/moontvplus/phone/update/UpdatePolicy.kt:72-74,94`、`apps/nsh-phone/App.tsx:16`、`apps/nsh-phone/embed/App.tsx:17`、`apps/nsh-phone/tools/dev-mirror.mjs:5`）。
   现状把 `https://tv.370125.xyz` 写死在客户端里，`UpdatePolicy.REPOSITORY` 也指向前账号。要改必须知道你的站点域名，两个可选方向：
   - 你有自己的域名 → 我把 `REPOSITORY` 改成 `NingKingStar/MoonTVPlus`、`SITE_BASE_URL`/`SITE_APK_URL`/`DEFAULT_SERVER` 改成你的域名，相关 Kotlin 测试里的固定 URL 一起改。
   - 你还没有域名 → 更稳的做法是把「站点源」改成构建参数（Gradle `buildConfigField`）默认关闭，只保留 GitHub Release 更新；这会给 `UpdatePolicy.kt`、`SiteUpdateSource`、Gradle、3 个测试文件带来改动。
   这两种都需要能编译 Kotlin 才能验证，而本机没有 Android 构建环境，所以我没有动，避免交一份编译不过的改动。
2. **镜像首次发布**：见第 3 节第 1 条。在跑之前，README 里的 `ghcr.io/ningkingstar/moontvplus` 一定是 404（已在 README 明示）。
3. **`LICENSE` 与 README 授权声明不一致**（MIT vs CC BY-NC-SA 4.0）：这属于法律口径，需要你或版权所有者决定，我没有擅自改。
