// Controlled upstream "片源" used by the isolated contract run.
//
// This is not a real third-party provider: it speaks the MacCMS videolist
// JSON shape the instance already consumes, and it serves the repository's
// generated test media so no external network is involved. It exists so the
// instance side of the contract (auth, source filtering, episode table,
// proxy rewrite) can be exercised without touching production data.
//
// Usage: node fixture-source.mjs [--port 0] [--media <dir>]
// Prints one JSON line with the bound port and the two source bases.

import { createServer } from 'node:http';
import { createReadStream, existsSync, readFileSync, statSync } from 'node:fs';
import path from 'node:path';

const args = process.argv.slice(2);
function arg(name, fallback) {
  const index = args.indexOf(`--${name}`);
  return index >= 0 && args[index + 1] ? args[index + 1] : fallback;
}

const mediaRoot = path.resolve(
  arg('media', path.join(import.meta.dirname, '..', '..', 'media'))
);
const port = Number(arg('port', '0'));
// A slow source makes the incremental-vs-complete order visible; the hold
// instance used for UI screenshots stretches it further.
const slowMs = Number(process.env.FIXTURE_SLOW_MS || 1500);

const MIME = {
  '.m3u8': 'application/vnd.apple.mpegurl',
  '.ts': 'video/mp2t',
  '.m4s': 'video/iso.segment',
  '.mp4': 'video/mp4',
  '.bin': 'application/octet-stream',
  '.json': 'application/json; charset=utf-8',
};

// Episodes must end with ".m3u8" for the instance's episode table to accept
// them; that rule is part of what this fixture is here to keep visible.
function catalog(base) {
  return [
    {
      vod_id: 1001,
      vod_name: '回声档案',
      vod_pic: `${base}/media/poster.svg`,
      vod_remarks: '更新至 2 集',
      vod_year: '2026',
      type_name: '科幻',
      vod_class: '科幻 / 悬疑',
      vod_content: '<p>一段来自废弃中继站的录音。</p><p>受控测试内容。</p>',
      vod_play_url: [
        `第 1 集$http://127.0.0.1:PORT/media/ts/index.m3u8`,
        `第 2 集$http://127.0.0.1:PORT/media/fmp4/index.m3u8`,
      ].join('#'),
    },
    {
      vod_id: 1002,
      vod_name: '暮色电台',
      vod_pic: `${base}/media/poster.svg`,
      vod_remarks: '单集',
      vod_year: '2026',
      type_name: '剧情',
      vod_class: '剧情 / 音乐',
      vod_content: '午夜频道只剩一位主播。',
      vod_play_url: '第 1 集$http://127.0.0.1:PORT/media/master.m3u8',
    },
    {
      // Same title, different year: the client must not merge these.
      vod_id: 1003,
      vod_name: '回声档案',
      vod_pic: `${base}/media/poster.svg`,
      vod_remarks: '老版',
      vod_year: '2020',
      type_name: '科幻',
      vod_class: '科幻',
      vod_content: '同名不同年，用于核对合并规则。',
      vod_play_url: '第 1 集$http://127.0.0.1:PORT/media/ts/index.m3u8',
    },
    {
      // MP4-only episode list: the instance drops it, so search must not
      // report it as playable. Kept to make that rule visible.
      vod_id: 1004,
      vod_name: '只有 MP4 的条目',
      vod_pic: '',
      vod_year: '2025',
      type_name: '纪录',
      vod_class: '纪录',
      vod_content: '',
      vod_play_url: `第 1 集$http://127.0.0.1:PORT/media/clip.mp4`,
    },
  ];
}

function withPort(list, selfBase) {
  return JSON.parse(JSON.stringify(list).replaceAll('127.0.0.1:PORT', selfBase));
}

function json(res, status, body) {
  const text = JSON.stringify(body);
  res.writeHead(status, {
    'Content-Type': 'application/json; charset=utf-8',
    'Content-Length': Buffer.byteLength(text),
    'Cache-Control': 'no-store',
  });
  res.end(text);
}

function serveMedia(req, res, url) {
  const rel = decodeURIComponent(url.pathname.replace(/^\/media\//, ''));
  const file = path.join(mediaRoot, rel);
  if (!file.startsWith(mediaRoot) || !existsSync(file) || !statSync(file).isFile()) {
    json(res, 404, { error: 'no such media' });
    return;
  }
  const type = MIME[path.extname(file)] || 'application/octet-stream';
  if (path.basename(file) === 'poster.svg') {
    const svg =
      '<svg xmlns="http://www.w3.org/2000/svg" width="300" height="420"><rect width="300" height="420" fill="#1b2432"/><text x="150" y="210" fill="#8fb3ff" font-size="22" text-anchor="middle">fixture</text></svg>';
    res.writeHead(200, { 'Content-Type': 'image/svg+xml', 'Content-Length': Buffer.byteLength(svg) });
    res.end(svg);
    return;
  }
  const size = statSync(file).size;
  res.writeHead(200, { 'Content-Type': type, 'Content-Length': size, 'Accept-Ranges': 'bytes' });
  createReadStream(file).pipe(res);
}

const server = createServer((req, res) => {
  const selfBase = `http://127.0.0.1:${server.address().port}`;
  const url = new URL(req.url, selfBase);

  if (url.pathname.startsWith('/media/')) {
    serveMedia(req, res, url);
    return;
  }

  // Bangumi calendar shape the instance proxies. One item has no Chinese
  // name, so the client's fallback to the original title stays visible.
  if (url.pathname === '/bangumi/calendar') {
    const days = [
      ['monday', '星期一'],
      ['tuesday', '星期二'],
      ['wednesday', '星期三'],
      ['thursday', '星期四'],
      ['friday', '星期五'],
      ['saturday', '星期六'],
      ['sunday', '星期日'],
    ].map(([en, cn], index) => ({
      weekday: { en, cn, ja: cn, id: index + 1 },
      items: [
        {
          id: 9000 + index * 2 + 1,
          name: `Fixture Anime ${index + 1}`,
          name_cn: `夹具番剧 ${index + 1}`,
          air_date: '2026-01-05',
          air_weekday: index + 1,
          images: { large: `${selfBase}/media/poster.svg`, common: `${selfBase}/media/poster.svg` },
          rating: { score: 7.5 },
        },
        {
          id: 9000 + index * 2 + 2,
          name: `Original Only ${index + 1}`,
          name_cn: '',
          air_date: '2025-10-06',
          air_weekday: index + 1,
          images: { large: `${selfBase}/media/poster.svg` },
          rating: { score: 0 },
        },
      ],
    }));
    json(res, 200, days);
    return;
  }

  const match = url.pathname.match(/^\/fixture-(a|b|c)\/api\.php\/provide\/vod\/?$/);
  if (!match) {
    json(res, 404, { error: 'unknown fixture path', path: url.pathname });
    return;
  }

  if (match[1] === 'b') {
    // A source that always fails: SSE must report source_error, JSON search
    // must stay empty instead of failing the whole request.
    json(res, 500, { error: 'fixture-b always fails' });
    return;
  }

  if (match[1] === 'c') {
    // A slow source that returns nothing after the fast source answered.
    // It keeps the incremental-vs-complete ordering observable.
    setTimeout(() => {
      json(res, 200, { code: 1, msg: '数据列表', page: 1, pagecount: 0, limit: '20', total: 0, list: [] });
    }, slowMs);
    return;
  }

  const ac = url.searchParams.get('ac');
  const list = withPort(catalog(selfBase), selfBase);
  if (ac === 'videolist' && url.searchParams.get('wd')) {
    json(res, 200, { code: 1, msg: '数据列表', page: 1, pagecount: 1, limit: '20', total: list.length, list });
    return;
  }
  if (ac === 'videolist' && url.searchParams.get('ids')) {
    const ids = String(url.searchParams.get('ids')).split(',');
    const picked = list.filter((item) => ids.includes(String(item.vod_id)));
    if (!picked.length) {
      json(res, 200, { code: 0, msg: '无数据', list: [] });
      return;
    }
    json(res, 200, { code: 1, msg: '数据列表', page: 1, pagecount: 1, total: picked.length, list: picked });
    return;
  }
  json(res, 400, { error: 'unsupported fixture query', ac });
});

server.listen(port, '127.0.0.1', () => {
  const bound = server.address().port;
  const base = `http://127.0.0.1:${bound}`;
  process.stdout.write(
    JSON.stringify({
      port: bound,
      base,
      fixtureA: `${base}/fixture-a/`,
      fixtureB: `${base}/fixture-b/`,
      mediaRoot,
      mediaPresent: existsSync(path.join(mediaRoot, 'ts', 'index.m3u8')),
    }) + '\n'
  );
});

for (const signal of ['SIGINT', 'SIGTERM']) {
  process.on(signal, () => server.close(() => process.exit(0)));
}
