import { cli, Strategy } from '@jackwener/opencli/registry';
import {
    ArgumentError,
    AuthRequiredError,
    CommandExecutionError,
    EmptyResultError,
} from '@jackwener/opencli/errors';

const CHATGPT_URL = 'https://chatgpt.com';
const CONVERSATION_ID_RE = /^[A-Za-z0-9_-]{8,}$/;

function unwrap(value) {
    if (value && !Array.isArray(value) && typeof value === 'object' && 'data' in value) {
        return value.data;
    }
    return value;
}

function parseConversationId(input) {
    const value = String(input || '').trim();
    if (CONVERSATION_ID_RE.test(value)) return value;
    try {
        const url = new URL(value, CHATGPT_URL);
        if (url.protocol !== 'https:' || url.hostname !== 'chatgpt.com') return '';
        return url.pathname.match(/^\/c\/([A-Za-z0-9_-]{8,})$/)?.[1] || '';
    } catch {
        return '';
    }
}

async function currentUrl(page) {
    const value = unwrap(await page.evaluate('window.location.href').catch(() => ''));
    return typeof value === 'string' ? value : '';
}

async function requireLoggedIn(page) {
    const url = await currentUrl(page);
    if (/\/(?:auth\/)?(?:login|signin)(?:[/?#]|$)/i.test(url)) {
        throw new AuthRequiredError('chatgpt.com', 'ChatGPT requires a logged-in browser session.');
    }
}

async function waitForCount(page, selector, attempts = 30) {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        const count = unwrap(await page.evaluate(
            `document.querySelectorAll(${JSON.stringify(selector)}).length`,
        ));
        if (Number(count) > 0) return Number(count);
        await page.wait(0.5);
    }
    return 0;
}

async function openRecentChats(page) {
    await page.goto(`${CHATGPT_URL}/`, { waitUntil: 'none' });
    await page.wait(1);
    await requireLoggedIn(page);
    await page.evaluate(`(() => {
      const normalize = (value) => String(value || '').replace(/\\s+/g, ' ').trim().toLowerCase();
      const labels = ['最近聊天', 'recent chats', 'recent', 'history', 'chats'];
      const buttons = Array.from(document.querySelectorAll('button'));
      const sidebar = buttons.find((button) => {
        const label = normalize(button.getAttribute('aria-label') || button.textContent);
        return label === '打开边栏' || label === 'open sidebar';
      });
      if (sidebar instanceof HTMLElement) sidebar.click();
      const recent = buttons.find((button) => {
        const label = normalize(button.getAttribute('aria-label') || button.textContent);
        return labels.includes(label);
      });
      if (recent instanceof HTMLElement) recent.click();
      return true;
    })()`);
    await page.wait(1);
}

cli({
    site: 'chatgpt',
    name: 'history-full',
    access: 'read',
    description: 'Return a bounded ChatGPT conversation window from the recent-chat sidebar.',
    domain: 'chatgpt.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [{ name: 'limit', type: 'int', required: false, default: 20 }],
    columns: ['Index', 'Id', 'Title', 'Url'],
    func: async (page, kwargs) => {
        const limit = Number(kwargs.limit ?? 20);
        if (!Number.isInteger(limit) || limit < 1 || limit > 50) {
            throw new ArgumentError('limit', 'must be between 1 and 50');
        }
        await openRecentChats(page);
        await waitForCount(page, 'a[href*="/c/"]', 20);
        const rows = unwrap(await page.evaluate(`(() => {
          const seen = new Set();
          return Array.from(document.querySelectorAll('a[href*="/c/"]')).flatMap((link) => {
            const href = link.getAttribute('href') || '';
            const match = href.match(/\\/c\\/([^/?#]+)/);
            if (!match || seen.has(match[1])) return [];
            seen.add(match[1]);
            return [{
              Id: match[1],
              Title: (link.textContent || '').replace(/\\s+/g, ' ').trim() || '(untitled)',
              Url: new URL(href, location.origin).href,
            }];
          });
        })()`));
        if (!Array.isArray(rows) || !rows.length) {
            throw new EmptyResultError(
                'chatgpt history-full',
                'No conversation links were mounted after opening Recent chats.',
            );
        }
        return rows.slice(0, limit).map((row, index) => ({ Index: index + 1, ...row }));
    },
});

cli({
    site: 'chatgpt',
    name: 'detail-full',
    access: 'read',
    description: 'Return structured ChatGPT messages, citations, and attachment references.',
    domain: 'chatgpt.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [
        { name: 'id', positional: true, required: true, help: 'Conversation ID or /c/<id> URL' },
    ],
    columns: [
        'ConversationId',
        'Title',
        'Messages',
        'MessageCount',
        'CitationCount',
        'AttachmentCount',
        'Complete',
    ],
    func: async (page, kwargs) => {
        const conversationId = parseConversationId(kwargs.id);
        if (!conversationId) throw new ArgumentError('id', 'must be a ChatGPT conversation ID or URL');
        await page.goto(`${CHATGPT_URL}/c/${conversationId}`, { waitUntil: 'none' });
        await requireLoggedIn(page);
        const count = await waitForCount(page, '[data-message-author-role]', 30);
        if (!count) {
            throw new EmptyResultError(
                'chatgpt detail-full',
                `No message nodes were mounted for conversation ${conversationId}.`,
            );
        }
        const result = unwrap(await page.evaluate(`(() => {
          const normalize = (value) => String(value || '')
            .replace(/\\u00a0/g, ' ')
            .replace(/[ \\t]+\\n/g, '\\n')
            .replace(/\\n{3,}/g, '\\n\\n')
            .trim();
          const attachmentSelector = [
            'a[download]',
            'a[href*="/backend-api/files"]',
            'a[href*="files.oaiusercontent.com"]',
            '[data-testid*="attachment"] a[href]',
            '[data-testid*="file"] a[href]',
          ].join(',');
          const messages = Array.from(document.querySelectorAll('[data-message-author-role]'))
            .map((node, index) => {
              const role = node.getAttribute('data-message-author-role') || '';
              const content = node.querySelector('.markdown') || node;
              const text = normalize(content.textContent || '');
              const attachments = Array.from(node.querySelectorAll(attachmentSelector)).map((asset) => ({
                name: normalize(asset.getAttribute('download') || asset.textContent || asset.getAttribute('aria-label')),
                href: asset.href || '',
              })).filter((asset) => asset.href);
              const attachmentUrls = new Set(attachments.map((asset) => asset.href));
              const citations = Array.from(node.querySelectorAll('a[href]')).map((link) => ({
                text: normalize(link.textContent || link.getAttribute('aria-label')),
                href: link.href || '',
              })).filter((link) => link.href && !attachmentUrls.has(link.href));
              return { index, role, text, citations, attachments };
            })
            .filter((message) => message.role && message.text);
          const generating = !!document.querySelector(
            'button[data-testid="stop-button"], button[aria-label*="Stop"], button[aria-label*="停止"]',
          );
          return { title: document.title, messages, generating };
        })()`));
        if (!result || !Array.isArray(result.messages) || !result.messages.length) {
            throw new CommandExecutionError('ChatGPT returned malformed message structures', conversationId);
        }
        const citationCount = result.messages.reduce(
            (total, message) => total + message.citations.length,
            0,
        );
        const attachmentCount = result.messages.reduce(
            (total, message) => total + message.attachments.length,
            0,
        );
        return [{
            ConversationId: conversationId,
            Title: String(result.title || '(untitled)'),
            Messages: result.messages,
            MessageCount: result.messages.length,
            CitationCount: citationCount,
            AttachmentCount: attachmentCount,
            Complete: result.generating !== true,
        }];
    },
});
