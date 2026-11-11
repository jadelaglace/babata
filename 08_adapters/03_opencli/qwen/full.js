import { cli, Strategy } from '@jackwener/opencli/registry';
import {
    ArgumentError,
    CommandExecutionError,
    EmptyResultError,
} from '@jackwener/opencli/errors';

const QIANWEN_URL = 'https://www.qianwen.com';
const API_URL = 'https://chat2-api.qianwen.com/api/v1/session/msg/list';
const SESSION_ID_RE = /^[0-9a-f]{32}$/i;

function parseSessionId(input) {
    const value = String(input || '').trim();
    if (SESSION_ID_RE.test(value)) return value.toLowerCase();
    try {
        const url = new URL(value, `${QIANWEN_URL}/chat/`);
        if (url.protocol !== 'https:' || url.hostname !== 'www.qianwen.com') return '';
        return url.pathname.match(/^\/chat\/([0-9a-f]{32})\/?$/i)?.[1]?.toLowerCase() || '';
    } catch {
        return '';
    }
}

function unwrap(value) {
    return value && typeof value === 'object' && 'data' in value ? value.data : value;
}

function countMatchingKeys(value, matcher) {
    if (!value || typeof value !== 'object') return 0;
    if (Array.isArray(value)) {
        return value.reduce((total, item) => total + countMatchingKeys(item, matcher), 0);
    }
    let count = 0;
    for (const [key, child] of Object.entries(value)) {
        if (matcher(key.toLowerCase())) count += 1;
        count += countMatchingKeys(child, matcher);
    }
    return count;
}

function responseJson(result, sessionId) {
    const value = unwrap(result);
    if (!value || typeof value !== 'object') {
        throw new CommandExecutionError('Qianwen message response was empty', sessionId);
    }
    if (Number(value.status) !== 200) {
        throw new CommandExecutionError(
            `Qianwen message request returned HTTP ${value.status ?? 'unknown'}`,
            sessionId,
        );
    }
    let body;
    try {
        body = typeof value.body === 'string' ? JSON.parse(value.body) : value.body;
    } catch (error) {
        throw new CommandExecutionError(
            'Qianwen message response was not valid JSON',
            error instanceof Error ? error.message : String(error),
        );
    }
    if (!body || body.code !== 0) {
        throw new CommandExecutionError(
            `Qianwen message request returned code ${body?.code ?? 'unknown'}`,
            String(body?.message || body?.msg || sessionId),
        );
    }
    return body;
}

async function fetchMessages(page, sessionId) {
    await page.goto(`${QIANWEN_URL}/chat/${sessionId}`, { waitUntil: 'none' });
    await page.wait(1);
    const result = await page.evaluate(`(async () => {
      const userId = document.cookie.match(/(?:^|;\\s*)b-user-id=([^;]+)/)?.[1]
        || document.cookie.match(/(?:^|;\\s*)utdid=([^;]+)/)?.[1]
        || '';
      const params = new URLSearchParams({
        return_response_messages: 'true',
        session_id: ${JSON.stringify(sessionId)},
        biz_id: 'ai_qwen',
        event_filter: 'all',
        page_size: '100',
        chat_client: 'h5',
        device: 'pc',
        fr: 'pc',
        pr: 'qwen',
        ut: userId,
        la: 'zh-CN',
        tz: 'Asia/Shanghai',
        ve: '4.13.1',
      });
      const response = await fetch(${JSON.stringify(API_URL)} + '?' + params, {
        credentials: 'include',
      });
      return { status: response.status, body: await response.text() };
    })()`);
    return responseJson(result, sessionId);
}

cli({
    site: 'qwen',
    name: 'detail-full',
    access: 'read',
    description: 'Return one complete Qianwen conversation from the official session message API.',
    domain: 'www.qianwen.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [{ name: 'id', positional: true, required: true, help: 'Qianwen session ID or /chat/<id> URL' }],
    columns: [
        'SessionId',
        'Url',
        'Title',
        'Messages',
        'MessageCount',
        'HaveNextPage',
        'Complete',
        'ReferenceKeyCount',
        'AttachmentKeyCount',
    ],
    func: async (page, kwargs) => {
        const sessionId = parseSessionId(kwargs.id);
        if (!sessionId) throw new ArgumentError('id', 'must be a Qianwen session ID or URL');

        const body = await fetchMessages(page, sessionId);
        const data = body.data;
        if (!data || !Array.isArray(data.list)) {
            throw new CommandExecutionError(
                'Qianwen message response did not match the requested session',
                sessionId,
            );
        }
        if (typeof data.have_next_page !== 'boolean') {
            throw new CommandExecutionError('Qianwen response omitted have_next_page', sessionId);
        }
        if (data.have_next_page) {
            throw new CommandExecutionError(
                'Qianwen conversation is paginated; increase page_size or add pagination before capture',
                sessionId,
            );
        }
        if (!data.list.length) throw new EmptyResultError('qianwen detail-full', sessionId);
        if (data.list.some((row) => row?.session_id !== sessionId)) {
            throw new CommandExecutionError(
                'Qianwen message response contained a different session',
                sessionId,
            );
        }

        const title = String(
            unwrap(await page.evaluate('document.title').catch(() => '')) || sessionId,
        ).trim() || sessionId;
        return [{
            SessionId: sessionId,
            Url: `${QIANWEN_URL}/chat/${sessionId}`,
            Title: title,
            Messages: data.list,
            MessageCount: data.list.length,
            HaveNextPage: data.have_next_page,
            Complete: true,
            ReferenceKeyCount: countMatchingKeys(data.list, (key) => key.includes('ref') || key.includes('citation')),
            AttachmentKeyCount: countMatchingKeys(data.list, (key) => key.includes('attach') || key.includes('file') || key.includes('media') || key.includes('image')),
        }];
    },
});
