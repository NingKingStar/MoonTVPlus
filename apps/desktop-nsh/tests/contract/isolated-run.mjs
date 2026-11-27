// Isolated P0 contract run: a throwaway instance, a controlled upstream
// fixture, and the checks this client depends on.
//
// Everything it creates lives under --run-dir (a temp directory by default):
// its own SQLite file, Next build cache, server logs and the report. It never
// reads .data, never reuses an existing database, and only listens on
// 127.0.0.1.
//
// Usage (Windows, from anywhere):
//   node apps/desktop-nsh/tests/contract/isolated-run.mjs [--keep] [--skip-rust]
//
// Exit code 0 means every check passed.

import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';
import { createServer } from 'node:net';
import {
  appendFileSync,
  copyFileSync,
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import path from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

const require = createRequire(import.meta.url);
const here = import.meta.dirname;
const appRoot = path.resolve(here, '..', '..');
const repoRoot = path.resolve(appRoot, '..', '..');

const argv = process.argv.slice(2);
const flag = (name) => argv.includes(`--${name}`);
const opt = (name, fallback) => {
  const index = argv.indexOf(`--${name}`);
  return index >= 0 && argv[index + 1] ? argv[index + 1] : fallback;
};

const keep = flag('keep');
const skipRust = flag('skip-rust');
const stamp = new Date().toISOString().replace(/[-:]/g, '').replace(/\..+/, '');
const runDir = path.resolve(
  opt('run-dir', path.join(process.env.TEMP || '/tmp', `moontv-nsh-contract-${stamp}`))
);
const dataDir = path.join(runDir, 'data');
const dbPath = path.join(dataDir, 'moontv.db');
const logDir = path.join(runDir, 'logs');
const reportPath = path.join(runDir, 'report.json');

const USERNAME = opt('user', 'nsh-contract-owner');
const PASSWORD = opt('pass', `contract-${Math.random().toString(36).slice(2)}-${Date.now()}`);
const SECOND_USER = 'nsh-contract-banned';
const SECOND_PASS = 'banned-user-test';

const results = [];
const skipped = [];
const notes = [];

function record(id, title, ok, detail, evidence) {
  results.push({ id, title, ok, detail, evidence });
  const mark = ok ? 'PASS' : 'FAIL';
  console.log(`${mark} ${id}  ${title}${detail ? ` — ${detail}` : ''}`);
}

/// An external dependency (Douban's public API) was unreachable or timed
/// out. It is not this run's failure, but it is also not a pass, so it is
/// reported apart from the checks and never counted as verified.
function recordSkip(id, title, detail, evidence) {
  skipped.push({ id, title, detail, evidence });
  console.log(`SKIP ${id}  ${title}${detail ? ` — ${detail}` : ''}`);
}

function note(text) {
  notes.push(text);
  console.log(`note  ${text}`);
}

// ---------------------------------------------------------------- processes

function pipeTo(file, child) {
  const stream = createWriteStream(file, { flags: 'a' });
  child.stdout?.pipe(stream);
  child.stderr?.pipe(stream);
  return stream;
}

function killTree(pid) {
  return new Promise((resolve) => {
    if (!pid) return resolve();
    const child = spawn('taskkill', ['/pid', String(pid), '/T', '/F'], { stdio: 'ignore' });
    child.on('exit', () => resolve());
    child.on('error', () => resolve());
  });
}

async function freePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

async function waitForOk(url, timeoutMs, label) {
  const started = Date.now();
  let last = '';
  while (Date.now() - started < timeoutMs) {
    try {
      const response = await fetch(url, { headers: { 'user-agent': 'contract-probe' } });
      if (response.status < 500) return response;
      last = `HTTP ${response.status}`;
    } catch (error) {
      last = error.message;
    }
    await sleep(700);
  }
  throw new Error(`${label} did not answer within ${timeoutMs} ms (${last})`);
}

async function startNext(port, env) {
  const nextBin = path.join(repoRoot, 'node_modules', 'next', 'dist', 'bin', 'next');
  if (!existsSync(nextBin)) throw new Error(`next binary not found: ${nextBin}`);
  const child = spawn(process.execPath, [nextBin, 'dev', '-p', String(port), '-H', '127.0.0.1'], {
    cwd: repoRoot,
    env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  pipeTo(path.join(logDir, `next-${port}.log`), child);
  child.on('exit', (code) => {
    if (code && code !== 0) console.log(`next dev exited with ${code}`);
  });
  return child;
}

// ------------------------------------------------------------------- http

function client(base) {
  return async function call(method, pathname, options = {}) {
    const url = new URL(pathname, base).toString();
    const headers = { ...(options.headers || {}) };
    if (options.token) headers.authorization = `Bearer ${options.token}`;
    if (!headers['user-agent']) headers['user-agent'] = 'MoonTVPlus App/0.4 NSH (Windows)';
    let body;
    if (options.json !== undefined) {
      headers['content-type'] = 'application/json';
      body = JSON.stringify(options.json);
    } else if (options.text !== undefined) {
      body = options.text;
    }
    const response = await fetch(url, { method, headers, body, redirect: 'manual' });
    const text = await response.text();
    let parsed;
    try {
      parsed = text ? JSON.parse(text) : undefined;
    } catch {
      parsed = undefined;
    }
    return {
      status: response.status,
      headers: Object.fromEntries(response.headers.entries()),
      text,
      json: parsed,
    };
  };
}

function decodeToken(token) {
  try {
    return JSON.parse(decodeURIComponent(token));
  } catch {
    return null;
  }
}

function encodeToken(value) {
  return encodeURIComponent(JSON.stringify(value));
}

async function readSse(url, headers, timeoutMs = 45000) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  const events = [];
  const arrivals = [];
  const started = Date.now();
  try {
    const response = await fetch(url, { headers, signal: controller.signal });
    if (!response.ok) return { status: response.status, events, arrivals };
    const reader = response.body.getReader();
    const decoder = new TextDecoder();
    let buffer = '';
    for (;;) {
      const { value, done } = await reader.read();
      if (done) break;
      buffer += decoder.decode(value, { stream: true });
      let index;
      while ((index = buffer.indexOf('\n\n')) >= 0) {
        const frame = buffer.slice(0, index).trim();
        buffer = buffer.slice(index + 2);
        const line = frame.split('\n').find((l) => l.startsWith('data:'));
        if (!line) continue;
        try {
          const event = JSON.parse(line.slice(5).trim());
          events.push(event);
          arrivals.push({ type: event.type, source: event.source, at: Date.now() - started });
        } catch {
          const raw = line.slice(5).trim();
          events.push({ type: 'unparsed', raw });
          arrivals.push({ type: 'unparsed', at: Date.now() - started });
        }
      }
      if (events.some((e) => e.type === 'complete')) break;
    }
    return {
      status: response.status,
      events,
      arrivals,
      contentType: response.headers.get('content-type'),
    };
  } catch (error) {
    return {
      status: 0,
      events,
      arrivals,
      error: error.message,
      contentType: null,
    };
  } finally {
    clearTimeout(timer);
  }
}

// ------------------------------------------------------- instance lifecycle

function configureDatabase() {
  const Database = require(path.join(repoRoot, 'node_modules', 'better-sqlite3'));
  const db = new Database(dbPath);
  db.pragma('journal_mode = WAL');
  const row = db.prepare('SELECT config FROM admin_config WHERE id = 1').get();
  const config = row ? JSON.parse(row.config) : {};
  config.SourceConfig = [
    {
      key: 'fixture-a',
      name: '夹具 A',
      api: `${fixture.base}/fixture-a/api.php/provide/vod`,
      from: 'custom',
      disabled: false,
      weight: 40,
    },
    {
      key: 'fixture-b',
      name: '夹具 B',
      api: `${fixture.base}/fixture-b/api.php/provide/vod`,
      from: 'custom',
      disabled: false,
      weight: 10,
    },
    {
      key: 'fixture-c',
      name: '夹具 C（慢）',
      api: `${fixture.base}/fixture-c/api.php/provide/vod`,
      from: 'custom',
      disabled: false,
      weight: 20,
    },
  ];
  config.SpecialSourceApis = [];
  config.ClientAdSourceApis = ['fixture-a'];
  if (config.SiteConfig) {
    config.SiteConfig.LoginRequireTurnstile = false;
    config.SiteConfig.SearchDownstreamMaxPage = 2;
    // The Bangumi calendar is proxied through this base URL; point it at the
    // controlled fixture so the isolated run never calls bgm.tv.
    config.SiteConfig.BangumiApiBaseUrl = `${fixture.base}/bangumi`;
    config.SiteConfig.BangumiProxy = '';
    config.SiteConfig.BangumiDataSource = 'server-proxy';
  }
  const payload = JSON.stringify(config);
  if (row) {
    db.prepare('UPDATE admin_config SET config = ? WHERE id = 1').run(payload);
  } else {
    db.prepare('INSERT INTO admin_config (id, config) VALUES (1, ?)').run(payload);
  }

  // A second account that exists but is banned: permission-denied must be
  // distinguishable from an expired session.
  const crypto = require('node:crypto');
  const hash = crypto.createHash('sha256').update(SECOND_PASS).digest('hex');
  db.prepare(
    `INSERT OR REPLACE INTO users
       (username, password_hash, role, created_at, playrecord_migrated, favorite_migrated, skip_migrated, banned)
     VALUES (?, ?, 'user', ?, 1, 1, 1, 1)`
  ).run(SECOND_USER, hash, Date.now());
  db.close();
  return { sources: config.SourceConfig.map((s) => s.key), bannedUser: SECOND_USER };
}

function initDatabase(env) {
  return new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [path.join(repoRoot, 'scripts', 'init-sqlite.js')], {
      cwd: repoRoot,
      env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let out = '';
    child.stdout.on('data', (chunk) => (out += chunk));
    child.stderr.on('data', (chunk) => (out += chunk));
    pipeTo(path.join(logDir, 'init-sqlite.log'), child);
    child.on('error', reject);
    child.on('exit', (code) =>
      code === 0 ? resolve(out) : reject(new Error(`init-sqlite failed (${code})\n${out}`))
    );
  });
}

let fixture = null;
let nextDev = null;

async function main() {
  mkdirSync(dataDir, { recursive: true });
  mkdirSync(logDir, { recursive: true });
  console.log(`run dir: ${runDir}`);

  // 1. controlled upstream
  fixture = await new Promise((resolve, reject) => {
    const child = spawn(process.execPath, [path.join(here, 'fixture-source.mjs'), '--port', '0'], {
      cwd: here,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    pipeTo(path.join(logDir, 'fixture.log'), child);
    let buffer = '';
    const timer = setTimeout(() => reject(new Error('fixture did not start')), 15000);
    child.stdout.on('data', (chunk) => {
      buffer += chunk;
      const line = buffer.split('\n').find((l) => l.trim().startsWith('{'));
      if (line) {
        clearTimeout(timer);
        const info = JSON.parse(line);
        resolve({ ...info, child });
      }
    });
    child.on('error', reject);
  });
  console.log(`fixture upstream: ${fixture.base} (media present: ${fixture.mediaPresent})`);

  const env = {
    ...process.env,
    NEXT_PUBLIC_STORAGE_TYPE: 'd1',
    NEXT_PUBLIC_SITE_NAME: 'MoonTVPlus NSH Contract',
    SQLITE_DB_PATH: dbPath,
    USERNAME,
    PASSWORD,
    NEXT_TELEMETRY_DISABLED: '1',
    NODE_ENV: 'development',
  };

  // 2. isolated database from the repository's own migrations
  await initDatabase(env);
  writeFileSync(path.join(runDir, 'credentials.json'), JSON.stringify({ USERNAME, SECOND_USER }, null, 2));
  note('test databases are created only under this run directory; .data is never read');

  const port = await freePort();
  const base = `http://127.0.0.1:${port}/`;

  // 3. first start creates the default admin config, then it is rewritten
  nextDev = await startNext(port, env);
  await waitForOk(`${base}api/server-config`, 180000, 'first server start');
  await killTree(nextDev.pid);
  nextDev = null;
  await sleep(1200);
  const patched = configureDatabase();
  note(`configured sources: ${patched.sources.join(', ')}; banned account: ${patched.bannedUser}`);

  // 4. real run
  nextDev = await startNext(port, env);
  await waitForOk(`${base}api/server-config`, 180000, 'server start');

  const call = client(base);
  await runChecks({ call, base });
  if (!skipRust) await runRustContract(base);

  return { base, port };
}

// ------------------------------------------------------------------ checks

async function runChecks({ call, base }) {
  // ---- instance + login -------------------------------------------------
  let config = await call('GET', 'api/server-config');
  const configOk =
    config.status === 200 &&
    config.json?.StorageType === 'd1' &&
    config.json?.LoginRequireTurnstile === false;
  record(
    'C01',
    'server-config 提供 StorageType 与 Turnstile 开关',
    configOk,
    `HTTP ${config.status} StorageType=${config.json?.StorageType} turnstile=${config.json?.LoginRequireTurnstile}`,
    { status: config.status, StorageType: config.json?.StorageType }
  );

  const badPassword = await call('POST', 'api/login', {
    json: { username: USERNAME, password: 'definitely-wrong' },
  });
  record(
    'C02',
    '错误密码是 401，不泄露账号存在性',
    badPassword.status === 401,
    `HTTP ${badPassword.status} ${badPassword.json?.error || ''}`,
    { status: badPassword.status }
  );

  const emptyUser = await call('POST', 'api/login', { json: { password: 'x' } });
  record(
    'C03',
    '空用户名是参数错误 400',
    emptyUser.status === 400,
    `HTTP ${emptyUser.status} ${emptyUser.json?.error || ''}`,
    { status: emptyUser.status }
  );

  const login = await call('POST', 'api/login', { json: { username: USERNAME, password: PASSWORD } });
  const token = login.json?.token;
  const auth = login.json?.auth;
  const now = Date.now();
  const refreshAgeDays = auth?.refreshExpires ? (auth.refreshExpires - now) / 86400000 : -1;
  const loginOk =
    login.status === 200 &&
    login.json?.ok === true &&
    typeof token === 'string' &&
    auth?.username === USERNAME &&
    auth?.role === 'owner' &&
    typeof auth?.tokenId === 'string' &&
    typeof auth?.refreshToken === 'string';
  record(
    'C04',
    '密码登录返回 bearer token 与 owner 身份',
    loginOk,
    `HTTP ${login.status} role=${auth?.role} tokenId=${auth?.tokenId ? `${auth.tokenId.slice(0, 6)}…` : 'missing'}`,
    { status: login.status, role: auth?.role, hasRefresh: Boolean(auth?.refreshToken) }
  );

  const ageOk = refreshAgeDays > 59.9 && refreshAgeDays <= 60.01;
  record(
    'C05',
    'refresh 有效期是 60 天',
    ageOk,
    `${refreshAgeDays.toFixed(3)} 天`,
    { refreshAgeDays: Number(refreshAgeDays.toFixed(3)) }
  );

  const noToken = await call('GET', 'api/favorites');
  record('C06', '无凭据访问收藏是 401', noToken.status === 401, `HTTP ${noToken.status}`, {
    status: noToken.status,
  });

  const junkToken = await call('GET', 'api/favorites', { token: 'not-a-token' });
  record(
    'C07',
    '无法解析的 token 是 401',
    junkToken.status === 401,
    `HTTP ${junkToken.status}`,
    { status: junkToken.status }
  );

  // ---- search ----------------------------------------------------------
  const search = await call('GET', 'api/search?q=%E5%9B%9E%E5%A3%B0', { token });
  const items = search.json?.results || [];
  const byId = new Map(items.map((item) => [`${item.source}+${item.id}`, item]));
  const a1001 = byId.get('fixture-a+1001');
  const a1003 = byId.get('fixture-a+1003');
  const searchOk =
    search.status === 200 &&
    Boolean(a1001) &&
    a1001.title === '回声档案' &&
    a1001.year === '2026' &&
    a1001.source_name === '夹具 A' &&
    Array.isArray(a1001.episodes) &&
    a1001.episodes.length === 2 &&
    a1001.episodes.every((url) => url.endsWith('.m3u8')) &&
    a1001.episodes_titles?.length === 2 &&
    a1001.type_name === '科幻' &&
    a1001.proxyMode === false;
  record(
    'C08',
    '搜索返回完整条目：集表/标题/年份/类型/片源名',
    searchOk,
    `HTTP ${search.status} results=${items.length} episodes=${a1001?.episodes?.length} year=${a1001?.year}`,
    { status: search.status, count: items.length, episodeTitles: a1001?.episodes_titles }
  );

  const sameTitle = a1001 && a1003 && a1001.title === a1003.title && a1001.year !== a1003.year;
  record(
    'C09',
    '同名不同年作为不同条目返回（合并在客户端）',
    Boolean(sameTitle),
    `2026=${a1001?.year} 2020=${a1003?.year}`,
    { a1001: a1001?.year, a1003: a1003?.year }
  );

  const mp4OnlyDropped = !byId.has('fixture-a+1004');
  record(
    'C10',
    '集表只接受 .m3u8：仅 MP4 的条目被服务端丢弃',
    mp4OnlyDropped,
    mp4OnlyDropped ? '1004 未出现在结果中' : '1004 出现了，客户端会拿到不可播条目',
    { present: byId.has('fixture-a+1004') }
  );

  const emptySearch = await call('GET', 'api/search?q=', { token });
  record(
    'C11',
    '空查询返回空结果而不是错误',
    emptySearch.status === 200 && Array.isArray(emptySearch.json?.results),
    `HTTP ${emptySearch.status} results=${emptySearch.json?.results?.length}`,
    { status: emptySearch.status }
  );

  // ---- source detail ---------------------------------------------------
  const detail = await call('GET', 'api/source-detail?source=fixture-a&id=1001', { token });
  const detailOk =
    detail.status === 200 &&
    detail.json?.source === 'fixture-a' &&
    String(detail.json?.id) === '1001' &&
    detail.json?.episodes?.length === 2 &&
    detail.json?.episodes_titles?.length === 2 &&
    detail.json?.year === '2026' &&
    typeof detail.json?.desc === 'string' &&
    !detail.json.desc.includes('<p>');
  record(
    'C12',
    'source-detail 返回实时集表、年份与去标签简介',
    detailOk,
    `HTTP ${detail.status} episodes=${detail.json?.episodes?.length} descLen=${detail.json?.desc?.length}`,
    { status: detail.status, episodes: detail.json?.episodes }
  );

  const uaApp = await call('GET', 'api/source-detail?source=fixture-a&id=1001', {
    token,
    headers: { 'user-agent': 'MoonTVPlus App/0.4 NSH (Windows)' },
  });
  const proxied = (uaApp.json?.episodes || []).every((url) => String(url).includes('/api/proxy-m3u8?url='));
  record(
    'C13',
    'APP UA + 已配置源：集地址改写为站内去广告代理',
    uaApp.status === 200 && proxied,
    `HTTP ${uaApp.status} episodes=${uaApp.json?.episodes?.length} proxied=${proxied}`,
    { vary: uaApp.headers.vary, first: uaApp.json?.episodes?.[0] }
  );

  const uaBrowser = await call('GET', 'api/source-detail?source=fixture-a&id=1001', {
    token,
    headers: { 'user-agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)' },
  });
  const raw = (uaBrowser.json?.episodes || []).every((url) => !String(url).includes('/api/proxy-m3u8'));
  record(
    'C14',
    '浏览器 UA 不改写，与 APP 行为分离',
    uaBrowser.status === 200 && raw,
    `HTTP ${uaBrowser.status} proxied=${!raw}`,
    { first: uaBrowser.json?.episodes?.[0] }
  );

  const noEpisodes = await call('GET', 'api/source-detail?source=fixture-a&id=1004', { token });
  record(
    'C15',
    '无可用集表的详情返回可识别错误（不是空 200）',
    noEpisodes.status >= 400,
    `HTTP ${noEpisodes.status} ${noEpisodes.json?.error || ''}`,
    { status: noEpisodes.status, error: noEpisodes.json?.error }
  );

  const badId = await call('GET', 'api/source-detail?source=fixture-a&id=../etc/passwd', { token });
  record(
    'C16',
    '非法视频 ID 被拒绝 400',
    badId.status === 400,
    `HTTP ${badId.status} ${badId.json?.error || ''}`,
    { status: badId.status }
  );

  const unknownSource = await call('GET', 'api/source-detail?source=nope&id=1001', { token });
  record(
    'C17',
    '未知片源被拒绝 400',
    unknownSource.status === 400,
    `HTTP ${unknownSource.status} ${unknownSource.json?.error || ''}`,
    { status: unknownSource.status }
  );

  // ---- SSE -------------------------------------------------------------
  const sse = await readSse(`${base}api/search/ws?q=%E5%9B%9E%E5%A3%B0`, {
    authorization: `Bearer ${token}`,
    'user-agent': 'MoonTVPlus App/0.4 NSH (Windows)',
    accept: 'text/event-stream',
  });
  const types = sse.events.map((event) => event.type);
  const startEvent = sse.events.find((event) => event.type === 'start');
  const okEvent = sse.events.find((event) => event.type === 'source_result' && event.source === 'fixture-a');
  const failEvent = sse.events.find((event) => event.type === 'source_error' && event.source === 'fixture-b');
  const completeEvent = sse.events.find((event) => event.type === 'complete');
  const sseOk =
    sse.status === 200 &&
    String(sse.contentType || '').includes('text/event-stream') &&
    startEvent?.totalSources === 3 &&
    okEvent?.results?.length === 3 &&
    completeEvent?.completedSources === 3 &&
    completeEvent?.totalResults === 3;
  record(
    'C18',
    'SSE 搜索：start / source_result / complete 与计数（3 源，含慢源）',
    sseOk,
    `HTTP ${sse.status} ct=${sse.contentType} types=${types.join(',')}`,
    {
      start: startEvent,
      okCount: okEvent?.results?.length,
      complete: completeEvent,
    }
  );

  // Incremental delivery: the fast source's results must be observable long
  // before the slow source answers and before `complete`. If the client only
  // saw the batch at the end, these arrival times would be identical.
  const firstArrival = sse.arrivals.find(
    (entry) => entry.type === 'source_result' && entry.source === 'fixture-a'
  );
  const slowArrival = sse.arrivals.find(
    (entry) => entry.type === 'source_result' && entry.source === 'fixture-c'
  );
  const completeArrival = sse.arrivals.find((entry) => entry.type === 'complete');
  const incrementalOk =
    Boolean(firstArrival) &&
    Boolean(slowArrival) &&
    Boolean(completeArrival) &&
    completeArrival.at - firstArrival.at >= 1000 &&
    slowArrival.at <= completeArrival.at;
  record(
    'S01',
    'SSE 增量：快源结果先于慢源与 complete 到达（不是等整批）',
    incrementalOk,
    `首源 +${firstArrival?.at ?? '?'}ms，慢源 +${slowArrival?.at ?? '?'}ms，complete +${completeArrival?.at ?? '?'}ms`,
    { arrivals: sse.arrivals }
  );

  // A source that answers 500 is reported as an empty source_result: the
  // instance never emits source_error for a failed upstream, only for a
  // timeout. Clients must not read "no results" as "source is broken".
  const failedAsEmpty = sse.events.find(
    (event) => event.type === 'source_result' && event.source === 'fixture-b'
  );
  record(
    'C18b',
    '上游 500 在 SSE 里只是空 source_result，没有 source_error',
    failedAsEmpty?.results?.length === 0 && !failEvent,
    `fixture-b results=${failedAsEmpty?.results?.length ?? 'absent'} source_error=${Boolean(failEvent)}`,
    { failedAsEmpty, failEvent }
  );

  // ---- suggestions + search history ------------------------------------
  const suggestions = await call(
    'GET',
    `api/search/suggestions?q=${encodeURIComponent('回')}`,
    { token }
  );
  const suggestionTexts = (suggestions.json?.suggestions || []).map((item) => item.text);
  const suggestionShapeOk =
    suggestions.status === 200 &&
    Array.isArray(suggestions.json?.suggestions) &&
    (suggestions.json?.suggestions || []).every(
      (item) => typeof item.text === 'string' && typeof item.type === 'string'
    );
  record(
    'G01',
    '搜索建议返回 suggestions[{text,type}]，且能从夹具首个源得到真实标题',
    suggestionShapeOk && suggestionTexts.includes('回声档案'),
    `HTTP ${suggestions.status} count=${suggestionTexts.length} first=${suggestionTexts[0] || 'none'}`,
    { suggestionTexts }
  );

  const emptySuggestions = await call(
    'GET',
    'api/search/suggestions?q=',
    { token }
  );
  record(
    'G02',
    '空查询的建议是空数组，不触发上游搜索',
    emptySuggestions.status === 200 &&
      Array.isArray(emptySuggestions.json?.suggestions) &&
      emptySuggestions.json.suggestions.length === 0,
    `HTTP ${emptySuggestions.status}`,
    { status: emptySuggestions.status }
  );

  const anonymousSuggestions = await call('GET', 'api/search/suggestions?q=%E5%9B%9E');
  record(
    'G03',
    '无凭据不能读取搜索建议',
    anonymousSuggestions.status === 401,
    `HTTP ${anonymousSuggestions.status}`,
    { status: anonymousSuggestions.status }
  );

  const emptyHistory = await call('GET', 'api/searchhistory', { token });
  record(
    'H01',
    '搜索历史初始为空数组',
    emptyHistory.status === 200 &&
      Array.isArray(emptyHistory.json) &&
      emptyHistory.json.length === 0,
    `HTTP ${emptyHistory.status} body=${JSON.stringify(emptyHistory.json)}`,
    { status: emptyHistory.status }
  );

  const addOne = await call('POST', 'api/searchhistory', {
    token,
    json: { keyword: '回声' },
  });
  const afterOne = await call('GET', 'api/searchhistory', { token });
  record(
    'H02',
    'POST 搜索历史返回最新列表，GET 读回同一关键词',
    addOne.status === 200 &&
      Array.isArray(addOne.json) &&
      addOne.json[0] === '回声' &&
      Array.isArray(afterOne.json) &&
      afterOne.json.includes('回声'),
    `HTTP ${addOne.status} list=${JSON.stringify(addOne.json)}`,
    { added: addOne.json }
  );

  const addTwo = await call('POST', 'api/searchhistory', {
    token,
    json: { keyword: '暮色' },
  });
  record(
    'H03',
    '最新搜索排在最前，旧关键词仍在',
    addTwo.status === 200 &&
      addTwo.json?.[0] === '暮色' &&
      addTwo.json?.includes('回声'),
    `list=${JSON.stringify(addTwo.json)}`,
    { list: addTwo.json }
  );

  const blankKeyword = await call('POST', 'api/searchhistory', {
    token,
    json: { keyword: '   ' },
  });
  record(
    'H04',
    '空白关键词被拒绝 400',
    blankKeyword.status === 400,
    `HTTP ${blankKeyword.status}`,
    { status: blankKeyword.status }
  );

  const delOne = await call(
    'DELETE',
    `api/searchhistory?keyword=${encodeURIComponent('回声')}`,
    { token }
  );
  const afterDeleteOne = await call('GET', 'api/searchhistory', { token });
  record(
    'H05',
    '按关键词删除只影响这一条',
    delOne.status === 200 &&
      delOne.json?.success === true &&
      !afterDeleteOne.json?.includes('回声') &&
      afterDeleteOne.json?.includes('暮色'),
    `HTTP ${delOne.status} left=${JSON.stringify(afterDeleteOne.json)}`,
    { left: afterDeleteOne.json }
  );

  const delAll = await call('DELETE', 'api/searchhistory', { token });
  const afterDeleteAll = await call('GET', 'api/searchhistory', { token });
  record(
    'H06',
    '不带关键词的 DELETE 清空全部搜索历史',
    delAll.status === 200 &&
      delAll.json?.success === true &&
      Array.isArray(afterDeleteAll.json) &&
      afterDeleteAll.json.length === 0,
    `HTTP ${delAll.status} left=${JSON.stringify(afterDeleteAll.json)}`,
    { left: afterDeleteAll.json }
  );

  const anonymousHistory = await call('GET', 'api/searchhistory');
  record(
    'H07',
    '无凭据不能读取搜索历史',
    anonymousHistory.status === 401,
    `HTTP ${anonymousHistory.status}`,
    { status: anonymousHistory.status }
  );

  // ---- Bangumi calendar (local fixture, configured base URL) ------------
  const calendar = await call('GET', 'api/bangumi/calendar', { token });
  const days = Array.isArray(calendar.json) ? calendar.json : [];
  const monday = days.find((day) => day.weekday?.en === 'monday');
  const calendarOk =
    calendar.status === 200 &&
    days.length === 7 &&
    monday?.weekday?.cn === '星期一' &&
    monday?.items?.[0]?.name_cn === '夹具番剧 1' &&
    monday?.items?.[1]?.name_cn === '' &&
    monday?.items?.[1]?.name === 'Original Only 1' &&
    Number(monday?.items?.[0]?.id) === 9001;
  record(
    'B01',
    'Bangumi 日历经实例配置的本地 base URL 读取，7 天与中英文名原样返回',
    calendarOk,
    `HTTP ${calendar.status} days=${days.length} monday=${monday?.weekday?.cn || '?'}`,
    { day: monday?.weekday, first: monday?.items?.[0], second: monday?.items?.[1] }
  );

  // ---- Douban route validation (no upstream needed) --------------------
  const missingCategory = await call('GET', 'api/douban/categories?kind=movie', { token });
  record(
    'D01',
    '豆瓣分类缺少 category/type 时是 400，不伪造结果',
    missingCategory.status === 400,
    `HTTP ${missingCategory.status} ${missingCategory.json?.error || ''}`,
    { status: missingCategory.status }
  );

  const badKind = await call(
    'GET',
    'api/douban/categories?kind=book&category=%E7%83%AD%E9%97%A8&type=%E5%85%A8%E9%83%A8',
    { token }
  );
  record(
    'D02',
    '豆瓣分类 kind 只接受 tv/movie',
    badKind.status === 400,
    `HTTP ${badKind.status} ${badKind.json?.error || ''}`,
    { status: badKind.status }
  );

  const badLimit = await call(
    'GET',
    'api/douban/categories?kind=movie&category=%E7%83%AD%E9%97%A8&type=%E5%85%A8%E9%83%A8&limit=0',
    { token }
  );
  record(
    'D03',
    '豆瓣分类 limit 越界是 400',
    badLimit.status === 400,
    `HTTP ${badLimit.status}`,
    { status: badLimit.status }
  );

  const missingRecommendKind = await call(
    'GET',
    'api/douban/recommends?limit=8&start=0',
    { token }
  );
  record(
    'D04',
    '豆瓣推荐缺少 kind 是 400',
    missingRecommendKind.status === 400,
    `HTTP ${missingRecommendKind.status}`,
    { status: missingRecommendKind.status }
  );

  // ---- Douban public data through the instance (external) --------------
  // This is NOT the isolated fixture: it proves the real route can reach
  // Douban and maps cards. When the network is unavailable it is reported
  // as skipped, never as verified.
  const externalCategories = await call(
    'GET',
    'api/douban/categories?kind=movie&category=%E7%83%AD%E9%97%A8&type=%E5%85%A8%E9%83%A8&limit=8&start=0',
    { token }
  );
  const categoryList = externalCategories.json?.list;
  if (externalCategories.status === 200 && Array.isArray(categoryList)) {
    record(
      'D05',
      '豆瓣热门电影（外网经实例）返回带 id/标题/海报的列表',
      categoryList.every(
        (item) => item.id && typeof item.title === 'string' && item.title.length > 0
      ),
      `HTTP 200 items=${categoryList.length} first=${categoryList[0]?.title || 'none'}`,
      { first: categoryList[0] }
    );
  } else {
    recordSkip(
      'D05',
      '豆瓣热门电影（外网经实例）',
      `HTTP ${externalCategories.status} ${externalCategories.json?.error || '外部不可达'}`,
      { status: externalCategories.status }
    );
  }

  const externalRecommend = await call(
    'GET',
    'api/douban/recommends?kind=tv&limit=8&start=0&category=%E5%8A%A8%E7%94%BB&format=%E7%94%B5%E8%A7%86%E5%89%A7&region=all&year=all&platform=all&sort=T&label=all',
    { token }
  );
  const recommendList = externalRecommend.json?.list;
  if (externalRecommend.status === 200 && Array.isArray(recommendList)) {
    record(
      'D06',
      '豆瓣番剧筛选（外网经实例）返回带年份/评分的列表',
      recommendList.every(
        (item) =>
          item.id &&
          typeof item.title === 'string' &&
          item.title.length > 0 &&
          (item.rate === '' || /^[0-9.]+$/.test(String(item.rate)))
      ),
      `HTTP 200 items=${recommendList.length} first=${recommendList[0]?.title || 'none'}`,
      { first: recommendList[0] }
    );
  } else {
    recordSkip(
      'D06',
      '豆瓣番剧筛选（外网经实例）',
      `HTTP ${externalRecommend.status} ${externalRecommend.json?.error || '外部不可达'}`,
      { status: externalRecommend.status }
    );
  }

  // ---- favorites -------------------------------------------------------
  const emptyFavorites = await call('GET', 'api/favorites', { token });
  const favoritePayload = {
    key: 'fixture-a+1001',
    favorite: {
      title: '回声档案',
      source_name: '夹具 A',
      cover: `${fixture.base}media/poster.svg`,
      poster: `${fixture.base}media/poster.svg`,
      year: '2026',
      search_title: '回声档案',
      total_episodes: 2,
      save_time: Date.now(),
      origin: 'vod',
      vod_remarks: '更新至 2 集',
    },
  };
  const saveFavorite = await call('POST', 'api/favorites', { token, json: favoritePayload });
  const afterSave = await call('GET', 'api/favorites', { token });
  const saved = afterSave.json?.['fixture-a+1001'];
  const favoriteOk =
    emptyFavorites.status === 200 &&
    Object.keys(emptyFavorites.json || {}).length === 0 &&
    saveFavorite.status === 200 &&
    saveFavorite.json?.success === true &&
    saved?.title === '回声档案' &&
    saved?.source_name === '夹具 A' &&
    saved?.year === '2026' &&
    saved?.total_episodes === 2 &&
    typeof saved?.save_time === 'number' &&
    saved?.origin === 'vod' &&
    saved?.cover === favoritePayload.favorite.cover;
  record(
    'C19',
    '收藏完整载荷往返：字段不丢、key 为 source+id',
    favoriteOk,
    `HTTP ${saveFavorite.status} fields=${Object.keys(saved || {}).join(',')}`,
    { saved }
  );

  const single = await call('GET', 'api/favorites?key=fixture-a%2B1001', { token });
  record(
    'C20',
    '按 key 读取单条收藏',
    single.status === 200 && single.json?.title === '回声档案',
    `HTTP ${single.status}`,
    { status: single.status }
  );

  const invalidFavorite = await call('POST', 'api/favorites', {
    token,
    json: { key: 'fixture-a+9', favorite: { title: 'x' } },
  });
  record(
    'C21',
    '收藏缺少 source_name 被拒绝 400',
    invalidFavorite.status === 400,
    `HTTP ${invalidFavorite.status}`,
    { status: invalidFavorite.status }
  );

  const badKeyFavorite = await call('POST', 'api/favorites', {
    token,
    json: { key: 'only-source', favorite: { title: 'x', source_name: 'y' } },
  });
  record(
    'C22',
    '收藏 key 缺少 +id 被拒绝 400',
    badKeyFavorite.status === 400,
    `HTTP ${badKeyFavorite.status}`,
    { status: badKeyFavorite.status }
  );

  // ---- play records ----------------------------------------------------
  const recordPayload = {
    key: 'fixture-a+1001',
    record: {
      title: '回声档案',
      source_name: '夹具 A',
      cover: `${fixture.base}media/poster.svg`,
      year: '2026',
      index: 2,
      total_episodes: 2,
      play_time: 37.25,
      total_time: 1201.5,
      save_time: Date.now(),
      search_title: '回声档案',
      origin: 'vod',
    },
  };
  const saveRecord = await call('POST', 'api/playrecords', { token, json: recordPayload });
  const records = await call('GET', 'api/playrecords', { token });
  const stored = records.json?.['fixture-a+1001'];
  const recordOk =
    saveRecord.status === 200 &&
    saveRecord.json?.success === true &&
    stored?.index === 2 &&
    stored?.play_time === 37.25 &&
    stored?.total_time === 1201.5 &&
    stored?.title === '回声档案' &&
    typeof stored?.save_time === 'number';
  record(
    'C23',
    '观看记录写入/读回：1 基集号与秒单位不失真',
    recordOk,
    `HTTP ${saveRecord.status} index=${stored?.index} play_time=${stored?.play_time} total_time=${stored?.total_time}`,
    { stored }
  );

  // The record table keeps a fixed column set; `origin` is not one of them.
  // Live TV records are still distinguishable by their `live_` source prefix,
  // which is what the client filters on.
  record(
    'C24',
    'origin 不落库，但 live_ 前缀可用于排除直播记录',
    stored !== undefined && !('origin' in (stored || {})),
    `origin=${stored?.origin ?? '(未保存)'} is_anime=${stored?.is_anime} is_duanju=${stored?.is_duanju}`,
    { savedColumns: Object.keys(stored || {}) }
  );

  const zeroIndex = await call('POST', 'api/playrecords', {
    token,
    json: { key: 'fixture-a+1001', record: { ...recordPayload.record, index: 0 } },
  });
  record(
    'C25',
    '集号 0 被拒绝（服务端要求 1 基）',
    zeroIndex.status === 400,
    `HTTP ${zeroIndex.status}`,
    { status: zeroIndex.status }
  );

  const update = await call('POST', 'api/playrecords', {
    token,
    json: {
      key: 'fixture-a+1001',
      record: { ...recordPayload.record, play_time: 300, index: 2, save_time: Date.now() },
    },
  });
  const afterUpdate = await call('GET', 'api/playrecords', { token });
  const merged = afterUpdate.json?.['fixture-a+1001'];
  record(
    'C26',
    '同 key 再次写入是覆盖更新，不产生重复条目',
    update.status === 200 &&
      Object.keys(afterUpdate.json || {}).length === 1 &&
      merged?.play_time === 300,
    `HTTP ${update.status} keys=${Object.keys(afterUpdate.json || {}).length} play_time=${merged?.play_time}`,
    { play_time: merged?.play_time }
  );

  // ---- refresh ---------------------------------------------------------
  const refresh = await call('POST', 'api/auth/refresh', { token });
  const nextToken = refresh.json?.token;
  const before = decodeToken(token);
  const after = nextToken ? decodeToken(nextToken) : null;
  const refreshOk =
    refresh.status === 200 &&
    typeof nextToken === 'string' &&
    after?.timestamp >= before?.timestamp &&
    after?.tokenId === before?.tokenId &&
    after?.refreshToken === before?.refreshToken &&
    after?.refreshExpires === before?.refreshExpires;
  record(
    'C27',
    '刷新保持 tokenId/refreshToken/60 天到期，只换 access 时间戳',
    refreshOk,
    `HTTP ${refresh.status} delta=${after && before ? after.timestamp - before.timestamp : 'n/a'} ms`,
    { status: refresh.status }
  );

  const afterRefreshCall = await call('GET', 'api/favorites', { token: nextToken });
  record(
    'C28',
    '刷新后的 token 立即可用于业务请求',
    afterRefreshCall.status === 200,
    `HTTP ${afterRefreshCall.status}`,
    { status: afterRefreshCall.status }
  );

  // A token whose access half is already past 4 h must still refresh while
  // the 60-day half is valid: this is what the client does at the 10 min mark.
  const aged = { ...after, timestamp: Date.now() - (4 * 60 * 60 * 1000 + 60_000) };
  const agedRefresh = await call('POST', 'api/auth/refresh', { token: encodeToken(aged) });
  record(
    'C29',
    '过期的 access 部分仍可在 60 天内刷新',
    agedRefresh.status === 200 && Boolean(agedRefresh.json?.token),
    `HTTP ${agedRefresh.status}`,
    { status: agedRefresh.status }
  );

  const expired = { ...after, refreshExpires: Date.now() - 1000 };
  const expiredRefresh = await call('POST', 'api/auth/refresh', { token: encodeToken(expired) });
  record(
    'C30',
    '60 天到期后刷新是 401',
    expiredRefresh.status === 401,
    `HTTP ${expiredRefresh.status} ${expiredRefresh.json?.error || ''}`,
    { status: expiredRefresh.status }
  );

  const tampered = { ...after, refreshToken: 'deadbeef'.repeat(8) };
  const tamperedRefresh = await call('POST', 'api/auth/refresh', { token: encodeToken(tampered) });
  record(
    'C31',
    '伪造 refreshToken 被拒绝 401',
    tamperedRefresh.status === 401,
    `HTTP ${tamperedRefresh.status}`,
    { status: tamperedRefresh.status }
  );

  // ---- 403 / 429 -------------------------------------------------------
  const proxyDenied = await call(
    'GET',
    'api/proxy/vod/m3u8?source=fixture-b&url=http%3A%2F%2F127.0.0.1%2Fmedia%2Fts%2Findex.m3u8',
    { token }
  );
  record(
    'C32',
    '未开启代理模式的源访问代理是 403（权限拒绝不是会话过期）',
    proxyDenied.status === 403,
    `HTTP ${proxyDenied.status} ${proxyDenied.json?.error || ''}`,
    { status: proxyDenied.status }
  );

  const bannedRefresh = await call('POST', 'api/auth/refresh', {
    token: encodeToken({
      username: SECOND_USER,
      role: 'user',
      timestamp: Date.now(),
      tokenId: 'banned-device',
      refreshToken: 'x',
      refreshExpires: Date.now() + 86400000,
    }),
  });
  record(
    'C33',
    '封禁账号刷新是 403',
    bannedRefresh.status === 403,
    `HTTP ${bannedRefresh.status} ${bannedRefresh.json?.error || ''}`,
    { status: bannedRefresh.status }
  );

  const banIp = '203.0.113.77';
  const banStatuses = [];
  let rateLimited = null;
  for (let i = 0; i < 6; i += 1) {
    const attempt = await call('POST', 'api/login', {
      json: { username: USERNAME, password: `wrong-${i}` },
      headers: { 'x-forwarded-for': banIp },
    });
    banStatuses.push({ status: attempt.status, retryAfter: attempt.headers['retry-after'] });
    if (attempt.status === 429) rateLimited = attempt;
  }
  record(
    'C34',
    '连续失败登录触发 429 并带 Retry-After',
    rateLimited !== null && Boolean(rateLimited.headers['retry-after']),
    `状态序列 ${banStatuses.map((s) => s.status).join('→')} retry-after=${rateLimited?.headers['retry-after']}`,
    { banStatuses, retryAfter: rateLimited?.headers['retry-after'] }
  );

  // ---- logout ----------------------------------------------------------
  const logout = await call('POST', 'api/logout', { token: nextToken });
  const afterLogout = await call('POST', 'api/auth/refresh', { token: nextToken });
  record(
    'C35',
    '退出登录后旧凭据不能刷新（撤销真实生效）',
    logout.status === 200 && afterLogout.status === 401,
    `logout=${logout.status} refresh=${afterLogout.status}`,
    { logout: logout.status, refresh: afterLogout.status }
  );

  // ---- cleanup of written data ----------------------------------------
  const login2 = await call('POST', 'api/login', { json: { username: USERNAME, password: PASSWORD } });
  const token2 = login2.json?.token;
  const deleteFavorite = await call('DELETE', 'api/favorites?key=fixture-a%2B1001', { token: token2 });
  const finalFavorites = await call('GET', 'api/favorites', { token: token2 });
  record(
    'C36',
    '按 key 删除收藏是幂等的，未清空其它数据',
    deleteFavorite.status === 200 && Object.keys(finalFavorites.json || {}).length === 0,
    `HTTP ${deleteFavorite.status} remaining=${Object.keys(finalFavorites.json || {}).length}`,
    { status: deleteFavorite.status }
  );

  const deleteRecord = await call('DELETE', 'api/playrecords?key=fixture-a%2B1001', { token: token2 });
  const finalRecords = await call('GET', 'api/playrecords', { token: token2 });
  record(
    'C37',
    '按 key 删除播放记录',
    deleteRecord.status === 200 && Object.keys(finalRecords.json || {}).length === 0,
    `HTTP ${deleteRecord.status} remaining=${Object.keys(finalRecords.json || {}).length}`,
    { status: deleteRecord.status }
  );
}

// -------------------------------------------------------------- rust client

async function runRustContract(base) {
  const cargo = process.env.CARGO_BIN || 'cargo';
  const child = spawn(cargo, ['test', '--test', 'live_contract', '--', '--nocapture', '--test-threads=1'], {
    cwd: appRoot,
    env: {
      ...process.env,
      MOONTV_CONTRACT_BASE: base,
      MOONTV_CONTRACT_USER: USERNAME,
      MOONTV_CONTRACT_PASS: PASSWORD,
      MOONTV_CONTRACT_FIXTURE_MEDIA: fixture.base,
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let out = '';
  child.stdout.on('data', (chunk) => {
    out += chunk;
  });
  child.stderr.on('data', (chunk) => {
    out += chunk;
  });
  const code = await new Promise((resolve) => child.on('exit', resolve));
  writeFileSync(path.join(logDir, 'cargo-live-contract.log'), out);
  console.log(out.trimEnd());
  record(
    'R01',
    '真实客户端代码走完登录/搜索/SSE 增量/建议/历史/详情/收藏/记录/日历/刷新/退出',
    code === 0,
    `cargo test --test live_contract exit=${code}`,
    { log: path.join(logDir, 'cargo-live-contract.log') }
  );
}

// -------------------------------------------------------------------- report

function writeReport(context) {
  const passed = results.filter((r) => r.ok).length;
  const failed = results.length - passed;
  const report = {
    generatedAt: new Date().toISOString(),
    baseline: '5b132342e7d36282e67edd73d188468d8c446ac6',
    kind: 'isolated-instance-contract',
    runDir,
    instance: context?.base || null,
    database: dbPath,
    fixture: fixture ? { base: fixture.base, mediaRoot: fixture.mediaRoot } : null,
    checks: results,
    skipped,
    notes,
    summary: {
      total: results.length,
      passed,
      failed,
      skipped: skipped.length,
    },
  };
  writeFileSync(reportPath, JSON.stringify(report, null, 2));

  const lines = [
    '# Isolated desktop contract run (P0 + P1 increments)',
    '',
    `- generated: ${report.generatedAt}`,
    `- instance: ${report.instance}`,
    `- database: ${dbPath}`,
    `- controlled upstream: ${fixture?.base}`,
    `- checks: ${passed}/${results.length} passed, ${failed} failed, ${skipped.length} skipped (external)`,
    '',
    '| id | check | result | detail |',
    '|---|---|---|---|',
    ...results.map(
      (r) => `| ${r.id} | ${r.title.replaceAll('|', '\\|')} | ${r.ok ? 'pass' : 'FAIL'} | ${String(r.detail || '').replaceAll('|', '\\|')} |`
    ),
    ...skipped.map(
      (r) => `| ${r.id} | ${r.title.replaceAll('|', '\\|')} | skip (external) | ${String(r.detail || '').replaceAll('|', '\\|')} |`
    ),
    '',
    '## What is isolated and what is external',
    '',
    '- isolated: this repository\'s Next.js routes, a throwaway SQLite file, loopback ports and the local fixture source/Bangumi calendar under this run directory.',
    '- external: D05/D06 call Douban\'s public API through the isolated instance. They are reported as skip when the network is unavailable and are never counted as verified.',
    '- not verified here: a real deployed instance, real third-party sources, Windows/Linux audio/video, DPI, packaging, CI or deployment.',
    '',
    '## Notes',
    '',
    ...notes.map((n) => `- ${n}`),
    '',
  ];
  writeFileSync(path.join(runDir, 'report.md'), lines.join('\n'));
  if (flag('copy-to')) {
    const target = opt('copy-to');
    mkdirSync(path.dirname(target), { recursive: true });
    copyFileSync(path.join(runDir, 'report.md'), target);
  }
  console.log(`\nreport: ${path.join(runDir, 'report.md')}`);
  console.log(`json:   ${reportPath}`);
  console.log(
    `checks: ${passed}/${results.length} passed, ${failed} failed, ${skipped.length} skipped`
  );
  return failed;
}

// ---------------------------------------------------------------------- main

let context = null;
try {
  context = await main();
} catch (error) {
  console.error(error);
  record('X00', '运行环境准备', false, error.message);
} finally {
  if (nextDev) await killTree(nextDev.pid);
  if (fixture?.child) await killTree(fixture.child.pid);
  const failed = writeReport(context);
  if (!keep) {
    // Keep databases and logs: they are the evidence for this run.
    console.log(`kept evidence (no --keep needed): ${runDir}`);
  }
  rmSync(path.join(repoRoot, 'apps', 'desktop-nsh', '.contract-tmp'), { recursive: true, force: true });
  process.exit(failed === 0 ? 0 : 1);
}
