import { cli, Strategy } from '@jackwener/opencli/registry';
import {
    ArgumentError,
    AuthRequiredError,
    CommandExecutionError,
    EmptyResultError,
} from '@jackwener/opencli/errors';

const YUQUE_URL = 'https://www.yuque.com';

function validDocumentUrl(input) {
    try {
        const url = new URL(String(input || ''), YUQUE_URL);
        if (url.protocol !== 'https:' || url.hostname !== 'www.yuque.com') return '';
        if (!/^\/(?:go\/doc\/\d+|[^/]+\/[^/]+\/[^/]+)\/?$/.test(url.pathname)) return '';
        return url.href;
    } catch {
        return '';
    }
}

async function currentUrl(page) {
    const value = await page.evaluate('window.location.href').catch(() => '');
    return typeof value === 'string' ? value : value?.data || '';
}

async function requireLoggedIn(page) {
    const url = await currentUrl(page);
    if (/\/login(?:[/?#]|$)/.test(url) || url === `${YUQUE_URL}/`) {
        throw new AuthRequiredError('www.yuque.com', 'Yuque requires a logged-in browser session.');
    }
}

cli({
    site: 'web',
    name: 'yuque-recent-full',
    access: 'read',
    description: 'Return a bounded list of recently edited Yuque documents from the dashboard.',
    domain: 'www.yuque.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [{ name: 'limit', type: 'int', required: false, default: 10 }],
    columns: ['Index', 'Id', 'Title', 'Owner', 'Book', 'Updated', 'Url'],
    func: async (page, kwargs) => {
        const limit = Number(kwargs.limit ?? 10);
        if (!Number.isInteger(limit) || limit < 1 || limit > 20) {
            throw new ArgumentError('limit', 'must be between 1 and 20');
        }
        await page.goto(`${YUQUE_URL}/dashboard`, { waitUntil: 'none' });
        await requireLoggedIn(page);
        for (let attempt = 0; attempt < 30; attempt += 1) {
            const count = await page.evaluate('document.querySelectorAll(\'a[href^="/go/doc/"]\').length');
            if (Number(count?.data ?? count) > 0) break;
            await page.wait(0.25);
        }
        const rows = await page.evaluate(`(() => {
          const seen = new Set();
          return Array.from(document.querySelectorAll('a[href^="/go/doc/"]')).flatMap((link) => {
            const match = link.getAttribute('href')?.match(/\\/go\\/doc\\/(\\d+)/);
            if (!match || seen.has(match[1])) return [];
            seen.add(match[1]);
            const row = link.closest('tr');
            const ownerLinks = Array.from(row?.querySelectorAll('a[href]') || []).filter((item) => item !== link);
            const owner = (ownerLinks[0]?.textContent || '').trim();
            const book = (ownerLinks[1]?.textContent || '').trim();
            const rawTitle = (link.textContent || '').trim();
            const suffix = owner && book ? owner + ' / ' + book : '';
            return [{
              Id: match[1],
              Title: suffix && rawTitle.endsWith(suffix) ? rawTitle.slice(0, -suffix.length).trim() : rawTitle,
              Owner: owner,
              Book: book,
              Updated: (row?.querySelector('time')?.getAttribute('datetime') || row?.cells?.[2]?.textContent || '').trim(),
              Url: new URL(link.getAttribute('href'), location.origin).href,
            }];
          });
        })()`);
        const values = Array.isArray(rows) ? rows : rows?.data;
        if (!Array.isArray(values) || !values.length) {
            throw new EmptyResultError('yuque recent-full', 'No recent document links were found.');
        }
        return values.slice(0, limit).map((row, index) => ({ Index: index + 1, ...row }));
    },
});

cli({
    site: 'web',
    name: 'yuque-detail-full',
    access: 'read',
    description: 'Return Yuque official Markdown export, rendered text/HTML, and exported media.',
    domain: 'www.yuque.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [{ name: 'url', positional: true, required: true }],
    columns: ['Url', 'Title', 'Text', 'Html', 'Images', 'ImageCount'],
    func: async (page, kwargs) => {
        const url = validDocumentUrl(kwargs.url);
        if (!url) throw new ArgumentError('url', 'must be a Yuque /go/doc or canonical document URL');
        await page.goto(url, { waitUntil: 'none' });
        await requireLoggedIn(page);
        for (let attempt = 0; attempt < 40; attempt += 1) {
            const result = await page.evaluate('Boolean(document.querySelector(".ne-viewer-body"))');
            if (Boolean(result?.data ?? result)) break;
            await page.wait(0.25);
        }
        const mounted = await page.evaluate('Boolean(document.querySelector(".ne-viewer-body"))');
        if (!Boolean(mounted?.data ?? mounted)) {
            throw new EmptyResultError('yuque detail-full', 'Yuque document body did not mount.');
        }
        const result = await page.evaluate(`(() => {
          const body = document.querySelector('.ne-viewer-body');
          const normalize = (value) => String(value || '').replace(/\\u00a0/g, ' ').replace(/\\n{3,}/g, '\\n\\n').trim();
          const canonicalUrl = location.href.replace(/\\/$/, '');
          return {
            url: canonicalUrl,
            title: (document.querySelector('h1')?.textContent || document.title || '').trim(),
            text: normalize(body.innerText || body.textContent),
            html: body.innerHTML,
          };
        })()`);
        const value = result?.data ?? result;
        if (!value?.text || !value?.html || !value?.url) {
            throw new CommandExecutionError(
                'Yuque rendered document was incomplete',
                'Retry after the document finishes loading.',
            );
        }
        const markdownUrl = `${value.url.replace(/\/$/, '')}/markdown?plain=true&linebreak=true&anchor=true`;
        await page.goto(markdownUrl, { waitUntil: 'none' });
        const markdownResult = await page.evaluate('document.body.innerText || document.body.textContent || ""');
        const markdown = String(markdownResult?.data ?? markdownResult).trim();
        if (!markdown) {
            throw new CommandExecutionError(
                'Yuque official Markdown export was empty',
                'Retry from the signed-in Yuque document page or use the official export UI.',
            );
        }
        const seen = new Set();
        const images = [];
        const mediaPattern = /!?\[([^\]]*)\]\((https:\/\/cdn\.nlark\.com\/yuque\/[^)\s]+)(?:\s+"[^"]*")?\)/g;
        for (const match of markdown.matchAll(mediaPattern)) {
            const mediaUrl = match[2].replace(/&amp;/g, '&');
            if (seen.has(mediaUrl)) continue;
            seen.add(mediaUrl);
            const token = mediaUrl.split('?')[0].split('/').pop() || '';
            images.push({ Index: images.length, Token: token, Url: mediaUrl, Alt: match[1] || '' });
        }
        return [{
            Url: value.url,
            Title: value.title,
            Text: value.text,
            Html: value.html,
            Markdown: markdown,
            Images: images,
            ImageCount: images.length,
        }];
    },
});
