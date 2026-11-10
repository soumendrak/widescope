import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const root = resolve(import.meta.dirname, '../../..');

describe('deployed unknown-path handling', () => {
  it('uses the Cloudflare 404 page instead of the SPA fallback', () => {
    const wrangler = readFileSync(resolve(root, 'wrangler.jsonc'), 'utf8');
    expect(wrangler).toMatch(/"not_found_handling"\s*:\s*"404-page"/);
    expect(wrangler).not.toContain('single-page-application');
    expect(readFileSync(resolve(root, 'ui/public/404.html'), 'utf8')).toContain('Page not found');
  });
});
