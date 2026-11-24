# Linux 隔离实例：版本、备份、回滚与本机验证边界

状态：文档 + 本地编排已就绪；**没有在真实 Linux 上执行过任何一步**。本文件的“未验证”结论优先于任何“应该可以”的推断。
适用：`apps/desktop-nsh` 的 P0 联调前置（roadmap P0-A 退出条件之一）。

## 1. 本轮实际有什么

|项目|实际状态|
|---|---|
|验证过的服务端|本仓 Next.js 路由代码，`next dev`，Windows x64，`NEXT_PUBLIC_STORAGE_TYPE=d1` + better-sqlite3|
|验证过的数据库|一次性 SQLite（`<run>/data/moontv.db`），由 `scripts/init-sqlite.js` 从本仓迁移生成|
|验证过的上游|本地受控夹具（MacCMS videolist JSON + 本仓生成媒体），无外网|
|Linux 实例|没有。没有地址、没有账号、没有镜像 digest、没有持久盘|
|生产数据|从未读取、从未迁移、从未写入|
|文档结论|Linux 上的文件系统语义、`/data` 权限、反代、健康检查、迁移回放都未验证|

因此本文件只定义“怎么做”和“做到什么算通过”，不能作为 Linux 已可用的证据。

## 2. 版本钉住（第一次上 Linux 前必须补齐）

在动任何数据之前，把下面这些写进隔离环境自己的记录文件（不进 Git，不进聊天记录，不含密钥明文）：

|字段|要求|
|---|---|
|服务端源码|本仓提交 SHA（当前基线 `5b132342e7d36282e67edd73d188468d8c446ac6`）；构建产物另存 `git rev-parse HEAD` 输出|
|镜像|`<registry>/<repo>@sha256:…` 摘要，不要只用 tag|
|存储后端|`NEXT_PUBLIC_STORAGE_TYPE` 的实际值（d1 / postgres / redis / kvrocks / upstash / turso）与数据库版本|
|Node|`node -v` 与 `pnpm-lock.yaml` 对应的依赖树|
|持久化目录|镜像内 `/data` 的实际挂载点、宿主机路径、容量与剩余空间|
|配置来源|环境变量清单（只记键名）、`admin_config` 的导出、脚本源文件|
|反代|域名、TLS 终止点、`X-Forwarded-*` 与 `SITE_BASE`/`NEXT_PUBLIC_SITE_URL` 的实际取值|
|加密材料|备份加密密钥的保管人和位置（只记位置，不记值）|

站点 `Version` 字段不能替代上述任何一项。

## 3. 备份：按实际后端各自做，不做“通用导出”

`localstorage` 不是完整服务端同步，不能当数据库用。Redis/Kvrocks、Upstash、D1、非 CF SQLite、Postgres、Turso 各按自身一致性策略：

|后端|备份动作|校验|
|---|---|---|
|SQLite（自建/非 CF）|停写或 `VACUUM INTO` 快照；连同 `-wal`/`-shm` 一致处理|在新路径打开快照，`SELECT count(*)` 核对 users/favorites/play_records|
|Postgres|`pg_dump -Fc` 或 PITR 基备|恢复到临时库后跑同一组计数查询|
|Redis/Kvrocks|`BGSAVE`/RDB + AOF，或 `redis-cli --rdb`|加载到临时实例后抽查 `user_tokens:*` 与配置键|
|Upstash / D1 / Turso|各自导出或时间旅行/分支|用导出物在隔离空间重建并抽查|
|文件|`tasks.json`、服务器下载目录、脚本源、订阅配置、主题/CSS|逐个文件存在性与大小；抽一个文件内容哈希|

必须一起备份：数据库 + 文件 + 配置 + 加密密钥 + 反代配置 + 镜像摘要。缺一项就不叫可回滚。

## 4. 恢复演练（对真实写联调放行的前提）

1. 在**隔离**主机/命名空间里，用同一镜像摘要 + 同一配置键名起来，挂载恢复出来的数据副本。
2. 只读校验：登录测试账号、读收藏/记录/设置、跑一次搜索、打开一次详情；不写入。
3. 写入校验：只写测试账号的收藏与观看记录，随后用同一份备份重建第二次，确认两次结果一致（说明备份可重复恢复）。
4. 记录：每步命令、耗时、失败点。任何一步失败就停，不带问题继续。

演练通过后才允许对隔离实例做真实写入联调。本机 Windows 上的 39 项契约检查**不能**替代这一步。

## 5. 回滚

- 迁移采用 expand/contract：先加表/加列并保持旧代码可读，再切代码，最后单独批准删除。失败即停，不回滚半个 migration。
- 回滚顺序：停写（或摘流量）→ 停定时任务 → 恢复同一时间点的数据库与文件 → 回退到记录过的镜像摘要 → 校验账号、收藏、观看记录、任务状态 → 恢复流量。
- `015` 一类删除性迁移不能靠回退容器恢复表：必须有迁移前的可恢复快照，且 `scripts/*` 的执行台账里写明执行时间与操作人。
- 桌面端启动/升级**不**执行任何生产库迁移；服务端迁移只由运维在受控环境里跑。
- 降级兼容：新客户端写出的字段必须能被旧服务端忽略；新 schema 不写回旧服务端。

## 6. 本机验证边界（现在能断言什么）

|能力|本机（Windows + next dev + SQLite）|Linux 真实实例|
|---|---|---|
|登录/刷新/撤销/401/403/429|已验证（39 项）|未验证|
|搜索 JSON / SSE 语义|已验证|未验证（反代缓冲、超时、`text/event-stream` 透传）|
|source-detail 集表与 UA 代理|已验证|未验证（`SITE_BASE`/反代 Host）|
|收藏/观看记录读写与合并|已验证（单进程）|未验证（并发、多副本、锁）|
|SQLite 迁移|已验证（`init-sqlite.js`）|未验证（文件权限、挂载、WAL on NFS）|
|Postgres/Redis/Turso|未验证|未验证|
|持久盘、备份、回滚|未做|未做|
|音画/解码/多屏/DPI|仅受控媒体播放，未做听感与硬解验证|未验证|

## 7. 把同一套契约检查指向 Linux 实例（后续）

编排脚本已支持外部传入地址，不需要改代码：

```bash
# 在隔离 Linux 实例上（Node 24，依赖可用）
MOONTV_CONTRACT_BASE=https://nsh-contract.example.invalid/ \
MOONTV_CONTRACT_USER=<测试账号> \
MOONTV_CONTRACT_PASS=<测试密码> \
node apps/desktop-nsh/tests/contract/isolated-run.mjs --skip-instance-setup
```

注意：

- 现在脚本只跑“自建一次性实例”模式；`--skip-instance-setup` 是后续要补的开关，当前版本会自己建库建源，**不能**指向已有实例，否则会改写它的来源配置。上真实实例前必须先加上这个分支并补一条“不写配置、只读来源”的保护。
- 指向真实实例时只允许专用测试账号与专用测试源；禁止用管理员账号做清除/迁移类操作。
- 测试结束必须能回答：写了哪些 key、是否已删除、是否影响其它账号。

## 8. 缺口清单（给下一轮）

1. 补 `--skip-instance-setup` 与“只读来源”保护，才能在已有实例上跑契约检查。
2. 真实 Linux 副本：镜像摘要、`/data` 备份、恢复演练记录。
3. 备份/回滚脚本与执行台账（谁、何时、哪份快照、结果）。
4. 反代对 SSE 与 `Range` 的实际行为（`proxy_m3u8` 依赖 Range）。
5. 多副本部署下的收藏/记录写入一致性（当前服务端是直接写库，没有 CAS）。
