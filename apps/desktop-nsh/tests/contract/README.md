# 隔离契约测试

这一组文件只做一件事：用**一次性**环境验证桌面对真实服务端的请求链，默认不联网、不碰任何现存数据库。

## 跑一次

```powershell
# 仓库根目录，node_modules 已安装，cargo 在 PATH
node apps/desktop-nsh/tests/contract/isolated-run.mjs
```

可选参数：

|参数|作用|
|---|---|
|`--skip-rust`|只跑 HTTP/SSE 检查，不调 `cargo test --test live_contract`|
|`--run-dir <path>`|指定一次性目录（默认 `%TEMP%\moontv-nsh-contract-<stamp>`）|
|`--user` / `--pass`|覆盖本次实例的站长账号（默认随机密码）|

退出码 0 表示全部检查通过。报告在 `<run-dir>\report.md` 与 `report.json`，日志与 SQLite 也在同一目录。

## 它创建了什么

1. `fixture-source.mjs`：本地受控“片源”，按 MacCMS videolist JSON 返回 4 个条目（含仅 MP4 的负例、同名不同年的两个条目、一个 500 源），媒体从 `apps/desktop-nsh/media` 读取。
2. `scripts/init-sqlite.js`：在 `<run-dir>\data\moontv.db` 建库，账号只存在于这个文件与本次进程环境。
3. `next dev`：`127.0.0.1:<随机端口>`，`NEXT_PUBLIC_STORAGE_TYPE=d1`。
4. 第一次启动后直接改写该库的 `admin_config`（来源 = 夹具 A/B，`ClientAdSourceApis = [fixture-a]`），再重启让配置生效；同时插入一个已封禁的测试账号用于 403。
5. 跑 38 项 HTTP/SSE 检查 + 1 项 Rust 客户端检查。

结束后 `taskkill` 掉两个子进程；目录保留作为证据。

## 它不会做什么

- 不读 `.data`、不读任何既有数据库、不改 `.env`。
- 不访问生产地址、不发外部网络请求（夹具与媒体都在本机）。
- 不写密钥：实例的 `USERNAME`/`PASSWORD` 只在子进程环境变量里，报告里只出现账号名。
- 不提交、不推送、不触发 CI。

## 与文档的关系

结论与未验证项见 [`docs/p0-contract-2026-11-24.md`](../../docs/p0-contract-2026-11-24.md)。
把同一套检查指向真实 Linux 实例的前置条件见 [`docs/nsh-linux-isolation-runbook.md`](../../../../docs/nsh-linux-isolation-runbook.md)（当前脚本只能自建实例，`--skip-instance-setup` 尚未实现）。
