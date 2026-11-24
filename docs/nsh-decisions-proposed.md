# 决策提案审阅副本

> 2026-11-24 更新：此为研究轮快照。两篇原文的现状：架构篇已转 `implemented/architecture/` 并落地 `apps/desktop-nsh`（单 crate，0.3.0）；CI 篇仍为 `proposed`，其补丁已在**本地工作区**应用（未提交、未推送、未触发 workflow，远端环境规则也未配置）。下面保留研究轮原文，便于对照当时的假设。

两篇原件位于被当前 .gitignore 忽略的 .agents/notes/proposed/；本副本保证 docs 审阅时不遗漏。Status 均为 proposed，没有落地代码或 CI。笔记互链在此副本映射为下方标题；格式门禁以原件为准。

---

原件：.agents/notes/proposed/architecture/2026-11-24-nsh-same-repo-rebuild.md

# Agent Note: 原仓全量功能的 Rust 与 NSH 重构边界

Status: proposed

## Problem

原仓同时包含网页、服务端、Android 与 Compose Desktop。把可运行原生演示当作完整客户端，或把同仓误解为继承网页界面，都会丢失账号权限、管理运维与复杂媒体能力。用户已明确要求继续使用原 GitHub 仓库，原功能全量纳入、Rust 与 NSH 重新设计实现。

## Proposal

建议在未占用的 apps/desktop-nsh 建独立 Cargo workspace；应用、领域状态、API、媒体、存储和平台适配分层。服务端继续独立发布，桌面只通过受鉴权接口访问。网页、Android 与现有 Compose Desktop 保留过渡。管理功能以原生管理界面加服务端和运维交付达到等值，不把网页过渡当永久终点。

以 docs/nsh-rebuild-requirements.md 的稳定编号管理全量范围；P0–P3 表示先后。C:/project/MoonTVPlus-NSH 仅只读参考，未来按文件审计后才迁入独立新写实现，默认不复制旧 App 代码。本轮只写需求与提案，未创建 workspace。

## 历史检索与约束

2026-11-24 检索 proposed、implemented、rejected 中的 NSH、同仓与发布主题，没有既有决策篇；仅存在 notes 的规则文件。独立新仓方案已被用户取代；目录与 crate 划分仍是待审提案。关联发布提案见 [CI 隔离](#agent-note-同仓桌面与服务端发布隔离)。

## Alternatives considered

- 独立新仓：能天然隔离服务端发布，依赖和权限更清晰；用户已要求原仓推进，不采用。原仓仍需补路径与发布门禁。
- 复用 Compose Desktop：已有共享模型、登录搜索页面和 JVM 基础，能减少重复劳动；桌面内嵌播放尚有占位，而且用户指定原生 Rust 与 NSH，不作为新实现基础。
- 把新 Rust workspace 放根目录：统一 Rust 工具入口直观；现有根目录承担 Next 服务端部署，新桌面依赖应局部隔离，先采用子目录 workspace，等第二个实际 Rust 应用出现再评估共享。

## Acceptance criteria

需求矩阵覆盖全部当前页面/API 与管理运维范围，明确历史缺失；mock、真实待联调、未做分开。未来实施时，桌面可独立构建，服务端可独立回滚；不同账号实例资料隔离；真实接口播放与记录同步通过后才报告兼容；原生管理完成前不得删除旧入口。

## Risks

独立实现会增加合同维护和媒体桥成本；NSH 固定依赖的多系统能力尚未证明。全量等值规模远超演示，需要按验收门逐项推进。服务端新增 handoff/capabilities/受控脚本执行要保持旧客户端兼容；不能把桌面升级变成强制生产迁移。笔记目录当前被忽略，审阅副本存 docs/nsh-decisions-proposed.md；未来追踪策略需单独审阅。

---

原件：.agents/notes/proposed/process/2026-11-24-nsh-ci-release-isolation.md

# Agent Note: 同仓桌面与服务端发布隔离

Status: proposed

## Problem

基线 5b132342e7d36282e67edd73d188468d8c446ac6 的 Cloudflare workflow 对 main push 无路径过滤；D1 分支遍历 SQL 并远程执行，失败后仍继续。桌面或研究文档的合入可能执行无关部署。Android 更新又扫描 client-v 标签，桌面混用会污染更新渠道。

## Proposal

先审阅 docs/nsh-ci-isolation.proposed.patch：排除纯 apps/desktop-nsh 与本轮 docs/nsh-* 路径 push，手动任务限 main，部署 job 引入 server-production 环境。管理员必须先实际配置该环境的审批与允许分支，单写环境名不构成保护。桌面 PR 仅检查和产 artifact；后续 desktop-v* 发布工作流独立签名、权限与目标，服务器版本独立。

首次隔离补丁本身会修改 workflow，仍可能触发原生产流程；必须先具备真实环境门禁，且不批准发布。本轮不应用补丁、不建工作流、不改仓库设置、不推送。后续应另行提出迁移台账、失败即停和部署后验证，不能宣称路径过滤解决数据库风险。

## 历史检索

2026-11-24 在活跃笔记树检索发布、Cloudflare、NSH，无既有同主题篇。关联目录边界见 [同仓重构](#agent-note-原仓全量功能的-rust-与-nsh-重构边界)。

## Alternatives considered

- 仅靠开发分支：不合 main 时不会触发 main push，现阶段隔离直接有效；合入后没有保护，故仍需发布门禁。
- 仅改为手动部署：最容易阻断 push 自动上线；会改变用户现有服务端发布习惯，作为可选更强方案，当前先提最小路径隔离加审批。
- 服务器路径白名单：新增桌面文件默认不会部署，长期最清晰；目前根目录配置、脚本和共享依赖较多，遗漏会让必要服务端变更不发布，需先建立服务端依赖路径清单，不能本轮草率替换。

## Acceptance criteria

补丁可对精确 HEAD 做 git apply --check；真实应用后用事件矩阵验证纯桌面/docs push 跳过、混合服务端 push 进入待审批、非 main 手动不部署、PR 无生产凭据、桌面 tag 不进入 Android 更新。此轮只完成静态提案和补丁可应用性检查。

## Risks

路径过滤不控制 workflow_dispatch，必须保留 ref 与真实环境门禁；环境保护可能受仓库方案限制。跳过工作流不宜成为必需状态检查，应另设总会返回结果的 PR 检查。迁移脚本与既有硬编码发布目标仍需单独修复；禁止以提交或合并同意替代上线授权。
