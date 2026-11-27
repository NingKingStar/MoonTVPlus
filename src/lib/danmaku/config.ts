import type { AdminConfig } from '@/lib/admin.types';

export const BUILTIN_DANMAKU_API_BASE =
  process.env.BUILTIN_DANMAKU_API_BASE?.trim() || '';
export const BUILTIN_DANMAKU_API_TOKEN =
  process.env.BUILTIN_DANMAKU_API_TOKEN?.trim() || '';

function trimTrailingSlash(value: string) {
  return value.replace(/\/+$/, '');
}

export function getDanmakuApiBaseUrl(siteConfig: AdminConfig['SiteConfig']) {
  if (siteConfig.DanmakuSourceType === 'builtin' && BUILTIN_DANMAKU_API_BASE) {
    return trimTrailingSlash(BUILTIN_DANMAKU_API_BASE);
  }

  const base = trimTrailingSlash(siteConfig.DanmakuApiBase || 'http://localhost:9321');
  const token = (siteConfig.DanmakuApiToken || BUILTIN_DANMAKU_API_TOKEN).trim();

  return token && token !== BUILTIN_DANMAKU_API_TOKEN
    ? `${base}/${token}`
    : base;
}
