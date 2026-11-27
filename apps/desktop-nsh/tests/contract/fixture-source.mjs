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
// Screenshot-only knob: make the mirror source's media fail so the client's
// automatic backup-source path can be seen without a real broken provider.
const badMirror = process.env.FIXTURE_BAD_MIRROR === '1';

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
      type_id: 1,
      vod_class: '科幻 / 悬疑',
      vod_play_from: 'm3u8',
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
      type_id: 2,
      vod_class: '剧情 / 音乐',
      vod_play_from: 'm3u8',
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
      type_id: 1,
      vod_class: '科幻',
      vod_play_from: 'm3u8',
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
      type_id: 1,
      vod_class: '纪录',
      vod_content: '',
      vod_play_from: 'm3u8',
      vod_play_url: `第 1 集$http://127.0.0.1:PORT/media/clip.mp4`,
    },
    {
      vod_id: 7001,
      vod_name: '短剧·甜宠一',
      vod_pic: `${base}/media/poster.svg`,
      vod_remarks: '更新至 3 集',
      vod_year: '2026',
      type_name: '短剧',
      type_id: 81,
      vod_class: '短剧 / 甜宠',
      vod_play_from: 'm3u8',
      vod_content: '<p>竖屏短剧夹具。</p>',
      vod_play_url: [
        `第 1 集$http://127.0.0.1:PORT/media/ts/index.m3u8`,
        `第 2 集$http://127.0.0.1:PORT/media/fmp4/index.m3u8`,
        `第 3 集$http://127.0.0.1:PORT/media/ts/index.m3u8`,
      ].join('#'),
    },
    {
      vod_id: 7002,
      vod_name: '短剧·逆袭二',
      vod_pic: `${base}/media/poster.svg`,
      vod_remarks: '单集',
      vod_year: '2025',
      type_name: '短剧',
      type_id: 82,
      vod_class: '短剧 / 逆袭',
      vod_play_from: 'm3u8',
      vod_content: '逆袭题材夹具。',
      vod_play_url: '第 1 集$http://127.0.0.1:PORT/media/master.m3u8',
    },
    {
      vod_id: 7003,
      vod_name: '短剧·甜宠三',
      vod_pic: '',
      vod_remarks: '',
      vod_year: '2024',
      type_name: '短剧',
      type_id: 81,
      vod_class: '短剧 / 甜宠',
      vod_play_from: 'm3u8',
      vod_content: '',
      vod_play_url: '第 1 集$http://127.0.0.1:PORT/media/range/index.m3u8',
    },
  ];
}

const CATEGORY_CLASSES = [
  { type_id: 1, type_name: '电影', type_pid: 0 },
  { type_id: 2, type_name: '剧集', type_pid: 0 },
  { type_id: 8, type_name: '短剧', type_pid: 0 },
  { type_id: 81, type_name: '短剧·甜宠', type_pid: 8 },
  { type_id: 82, type_name: '短剧·逆袭', type_pid: 8 },
];

function withPort(list, selfBase) {
  // The catalog writes `http://127.0.0.1:PORT/...`; inject only host:port so
  // the scheme is not duplicated.
  const host = selfBase.replace(/^https?:\/\//, '');
  return JSON.parse(JSON.stringify(list).replaceAll('127.0.0.1:PORT', host));
}

// A second source with the same title/year as fixture-a/1001. This is what
// makes source switching and "remember the chosen source" observable without
// a real third-party provider.
function mirrorCatalog(base) {
  return [
    {
      vod_id: 2001,
      vod_name: '回声档案',
      vod_pic: `${base}/media/poster.svg`,
      vod_remarks: '第二片源 · 2 集',
      vod_year: '2026',
      type_name: '科幻',
      type_id: 1,
      vod_class: '科幻 / 悬疑',
      vod_play_from: 'm3u8',
      vod_content: '<p>同一作品的第二片源夹具。</p>',
      vod_play_url: [
        `第 1 集$${badMirror ? 'http://127.0.0.1:PORT/media/missing.m3u8' : 'http://127.0.0.1:PORT/media/fmp4/index.m3u8'}`,
        `第 2 集$${badMirror ? 'http://127.0.0.1:PORT/media/missing2.m3u8' : 'http://127.0.0.1:PORT/media/ts/index.m3u8'}`,
      ].join('#'),
    },
  ];
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
    // A slow second source with the same work as fixture-a/1001. It has no
    // short-drama class, so the short-drama source list still excludes it.
    setTimeout(() => {
      const ac = url.searchParams.get('ac');
      const mirror = withPort(mirrorCatalog(selfBase), selfBase);
      if (ac === 'list') {
        json(res, 200, {
          code: 1,
          msg: '数据列表',
          class: [{ type_id: 1, type_name: '电影', type_pid: 0 }],
        });
        return;
      }
      if (ac === 'videolist') {
        const ids = url.searchParams.get('ids');
        const list = ids
          ? mirror.filter((item) => ids.split(',').includes(String(item.vod_id)))
          : mirror;
        json(res, 200, {
          code: 1,
          msg: '数据列表',
          page: Number(url.searchParams.get('pg') || 1),
          pagecount: 1,
          limit: '20',
          total: list.length,
          list,
        });
        return;
      }
      json(res, 200, { code: 1, msg: '数据列表', page: 1, pagecount: 0, limit: '20', total: 0, list: [] });
    }, slowMs);
    return;
  }

  const ac = url.searchParams.get('ac');
  const list = withPort(catalog(selfBase), selfBase);
  if (ac === 'list') {
    json(res, 200, { code: 1, msg: '数据列表', class: CATEGORY_CLASSES });
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
  if (ac === 'videolist') {
    const typeId = url.searchParams.get('t');
    const keyword = url.searchParams.get('wd');
    const paged = url.searchParams.has('pg');
    let filtered = list;
    if (typeId) {
      const wanted = String(typeId);
      filtered =
        wanted === '8'
          ? list.filter((item) => item.type_id === 81 || item.type_id === 82)
          : list.filter((item) => String(item.type_id) === wanted);
    } else if (keyword && paged) {
      filtered = list.filter((item) => String(item.vod_name).includes(keyword));
    } else if (keyword) {
      // Global search keeps the original contract set; short-drama rows are
      // reached through the short-drama/category routes instead.
      filtered = list.filter((item) => item.type_id === 1 || item.type_id === 2);
    }
    if (typeId || paged) {
      const page = Math.max(1, parseInt(url.searchParams.get('pg') || '1', 10) || 1);
      const pageSize = 2;
      const pageCount = Math.max(1, Math.ceil(filtered.length / pageSize));
      const pageItems = filtered.slice((page - 1) * pageSize, page * pageSize);
      json(res, 200, {
        code: 1,
        msg: '数据列表',
        page,
        pagecount: pageCount,
        limit: String(pageSize),
        total: filtered.length,
        list: pageItems,
      });
      return;
    }
    json(res, 200, { code: 1, msg: '数据列表', page: 1, pagecount: 1, limit: '20', total: filtered.length, list: filtered });
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
