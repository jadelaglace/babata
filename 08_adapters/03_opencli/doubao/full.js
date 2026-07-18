import { cli, Strategy } from '@jackwener/opencli/registry';
import {
    ArgumentError,
    CommandExecutionError,
    EmptyResultError,
} from '@jackwener/opencli/errors';

const DOUBAO_URL = 'https://www.doubao.com/';
const CONVERSATION_ID_RE = /^\d{8,}$/;

function parseConversationId(input) {
    const value = String(input || '').trim();
    if (CONVERSATION_ID_RE.test(value)) return value;
    try {
        const url = new URL(value, DOUBAO_URL);
        if (url.protocol !== 'https:' || url.hostname !== 'www.doubao.com') return '';
        return url.pathname.match(/^\/chat\/(\d{8,})$/)?.[1] || '';
    } catch {
        return '';
    }
}

function parseResponse(entry, operation) {
    if (!entry) throw new EmptyResultError('doubao full read', `${operation} was not captured.`);
    if (entry.responseBodyTruncated) {
        throw new CommandExecutionError(
            `${operation} exceeded the OpenCLI network capture limit`,
            `Captured ${entry.responseBodyFullSize || 'unknown'} characters.`,
        );
    }
    if (Number(entry.responseStatus || 0) !== 200) {
        throw new CommandExecutionError(
            `${operation} returned HTTP ${entry.responseStatus || 'unknown'}`,
            entry.url || '',
        );
    }
    let outer;
    try {
        outer = JSON.parse(String(entry.responsePreview || ''));
    } catch (error) {
        throw new CommandExecutionError(
            `${operation} returned invalid JSON`,
            error instanceof Error ? error.message : String(error),
        );
    }
    if (Number(outer.status_code || 0) !== 0) {
        throw new CommandExecutionError(
            `${operation} returned status ${outer.status_code}`,
            String(outer.status_desc || ''),
        );
    }
    try {
        return {
            outer,
            body: typeof outer.downlink_body === 'string'
                ? JSON.parse(outer.downlink_body)
                : outer.downlink_body,
        };
    } catch (error) {
        throw new CommandExecutionError(
            `${operation} returned an invalid downlink_body`,
            error instanceof Error ? error.message : String(error),
        );
    }
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

async function visibleConversationId(page) {
    const current = await page.evaluate('window.location.href').catch(() => '');
    const currentId = parseConversationId(current);
    if (currentId) return currentId;
    await page.goto(`${DOUBAO_URL}chat/`, { waitUntil: 'none' });
    await page.wait(2);
    return await page.evaluate(`(() => {
      const anchor = Array.from(document.querySelectorAll('a[href^="/chat/"]'))
        .find((item) => /^\/chat\/\d{8,}$/.test(item.getAttribute('href') || ''));
      return anchor?.getAttribute('href')?.match(/\/chat\/(\d{8,})/)?.[1] || '';
    })()`);
}

async function captureConversation(page, conversationId, pattern) {
    if (typeof page.startNetworkCapture !== 'function' || typeof page.readNetworkCapture !== 'function') {
        throw new CommandExecutionError(
            'OpenCLI Browser Bridge network capture is unavailable',
            'Install or update the OpenCLI Chrome extension.',
        );
    }
    await page.goto(`${DOUBAO_URL}chat/`, { waitUntil: 'none' });
    await page.wait(0.5);
    const started = await page.startNetworkCapture(pattern);
    if (!started) {
        throw new CommandExecutionError(
            'OpenCLI Browser Bridge network capture is unavailable',
            'Install or update the OpenCLI Chrome extension.',
        );
    }
    await page.goto(`${DOUBAO_URL}chat/${conversationId}`, { waitUntil: 'none' });
    await page.wait(3);
    return await page.readNetworkCapture();
}

cli({
    site: 'doubao',
    name: 'history-full',
    access: 'read',
    description: 'Return a bounded recent Doubao conversation window from the page network response.',
    domain: 'doubao.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [
        { name: 'limit', type: 'int', required: false, default: 20 },
    ],
    columns: ['Id', 'Title', 'Url', 'CreatedAt', 'UpdatedAt'],
    func: async (page, kwargs) => {
        const limit = Number(kwargs.limit);
        if (!Number.isInteger(limit) || limit < 1 || limit > 20) {
            throw new ArgumentError('limit', 'must be between 1 and 20');
        }
        const conversationId = await visibleConversationId(page);
        if (!conversationId) {
            throw new EmptyResultError('doubao history-full', 'No authenticated conversation was visible.');
        }
        const entries = await captureConversation(page, conversationId, '/im/chain/recent_conv');
        const entry = entries.find((item) => String(item?.url || '').includes('/im/chain/recent_conv'));
        const response = parseResponse(entry, 'recent_conv');
        const recent = response.body?.pull_recent_conv_chain_downlink_body;
        if (!recent || !Array.isArray(recent.cells)) {
            throw new CommandExecutionError('recent_conv did not return cells', conversationId);
        }
        return recent.cells.slice(0, limit).map((cell) => {
            const conversation = cell?.conversation || {};
            const id = String(conversation.conversation_id || cell.id || '');
            return {
                Id: id,
                Title: String(conversation.name || '(untitled)'),
                Url: `${DOUBAO_URL}chat/${id}`,
                CreatedAt: String(conversation.create_time || ''),
                UpdatedAt: String(conversation.update_time || ''),
            };
        });
    },
});

cli({
    site: 'doubao',
    name: 'detail-full',
    access: 'read',
    description: 'Return complete structured Doubao conversation info and message-chain responses.',
    domain: 'doubao.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [
        { name: 'id', positional: true, required: true, help: 'Conversation ID or full /chat/<id> URL' },
    ],
    columns: [
        'ConversationId',
        'Info',
        'Messages',
        'MessageCount',
        'HasMore',
        'MessageCursor',
        'AttachmentKeyCount',
        'MediaKeyCount',
        'ResponseBytes',
    ],
    func: async (page, kwargs) => {
        const conversationId = parseConversationId(kwargs.id);
        if (!conversationId) throw new ArgumentError('id', 'must be a Doubao conversation ID or URL');
        const entries = await captureConversation(
            page,
            conversationId,
            '/im/conversation/info|/im/chain/single',
        );
        const infoEntry = entries.find((item) => String(item?.url || '').includes('/im/conversation/info'));
        const chainEntry = entries.find((item) => String(item?.url || '').includes('/im/chain/single'));
        const infoResponse = parseResponse(infoEntry, 'conversation/info');
        const chainResponse = parseResponse(chainEntry, 'chain/single');
        const info = infoResponse.body?.get_conv_info_downlink_body;
        const chain = chainResponse.body?.pull_singe_chain_downlink_body;
        if (!info || !chain || !Array.isArray(chain.messages)) {
            throw new CommandExecutionError('Doubao returned incomplete conversation structures', conversationId);
        }
        return [{
            ConversationId: conversationId,
            Info: info,
            Messages: chain.messages,
            MessageCount: chain.messages.length,
            HasMore: chain.has_more === true,
            MessageCursor: String(chain.msg_cursor || ''),
            AttachmentKeyCount: countMatchingKeys(chain.messages, (key) => key.includes('attach') || key.includes('file')),
            MediaKeyCount: countMatchingKeys(chain.messages, (key) => key.includes('media') || key.includes('image') || key.includes('video') || key.includes('audio')),
            ResponseBytes: JSON.stringify(infoResponse.outer).length + JSON.stringify(chainResponse.outer).length,
        }];
    },
});
