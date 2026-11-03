import { cli, Strategy } from '@jackwener/opencli/registry';
import { ArgumentError, CommandExecutionError } from '@jackwener/opencli/errors';

const KIMI_URL = 'https://www.kimi.com/';
// Kimi chat IDs are 36-character UUIDs or, for older chats, 20-character base32hex XIDs.
const CHAT_ID_RE = /^(?:[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}|[0-9a-v]{20})$/i;
const MAX_MESSAGE_PAGES = 50;

function parseChatId(input) {
    const value = String(input || '').trim();
    if (CHAT_ID_RE.test(value)) return value.toLowerCase();
    try {
        const url = new URL(value, KIMI_URL);
        if (url.protocol !== 'https:' || url.hostname !== 'www.kimi.com') return '';
        const id = url.pathname.match(/^\/chat\/([0-9a-z-]+)$/i)?.[1] || '';
        return CHAT_ID_RE.test(id) ? id.toLowerCase() : '';
    } catch {
        return '';
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

// Read both payloads over the page's own session, so a chat that the UI never renders
// still has a complete, verifiable body. The session token stays inside the page.
async function readConversation(page, chatId) {
    const onKimi = await page.evaluate("location.hostname.endsWith('kimi.com')").catch(() => false);
    if (!onKimi) {
        await page.goto(KIMI_URL, { waitUntil: 'none' });
        await page.wait(1);
    }
    for (let attempt = 0; attempt < 2; attempt += 1) {
        const result = await page.evaluate(`(async () => {
            const token = localStorage.getItem('access_token');
            if (!token) return { authMissing: true };
            const post = async (method, body) => {
                const response = await fetch('/apiv2/kimi.gateway.chat.v1.ChatService/' + method, {
                    method: 'POST',
                    credentials: 'include',
                    headers: { 'Content-Type': 'application/json', Authorization: 'Bearer ' + token },
                    body: JSON.stringify(body),
                });
                const text = await response.text();
                let payload = null;
                try { payload = JSON.parse(text); } catch (error) { payload = null; }
                return { status: response.status, payload, text };
            };
            const chatResult = await post('GetChat', { chatId: ${JSON.stringify(chatId)} });
            if (chatResult.status === 404) return { removed: true };
            if (chatResult.status === 401) return { unauthorised: true };
            if (!chatResult.payload) return { error: 'GetChat returned non-JSON', status: chatResult.status };
            const messages = [];
            const pages = [];
            let pageToken = '';
            let guard = 0;
            do {
                const messageResult = await post('ListMessages', {
                    chatId: ${JSON.stringify(chatId)}, pageSize: 100, pageToken,
                });
                if (messageResult.status === 404) return { removed: true };
                if (messageResult.status === 401) return { unauthorised: true };
                if (!messageResult.payload) {
                    return { error: 'ListMessages returned non-JSON', status: messageResult.status };
                }
                const body = messageResult.payload;
                const batch = Array.isArray(body.messages) ? body.messages : [];
                messages.push(...batch);
                pages.push({ count: batch.length, response: body });
                pageToken = body.nextPageToken || body.next_page_token || '';
                guard += 1;
            } while (pageToken && guard < ${MAX_MESSAGE_PAGES});
            return {
                chat: chatResult.payload.chat,
                chatBytes: JSON.stringify(chatResult.payload).length,
                pages,
                messages,
                exhausted: !pageToken,
            };
        })()`);
        if (result && result.unauthorised && attempt === 0) {
            await page.wait(2);
            continue;
        }
        return result || { error: 'page returned no result' };
    }
    return { unauthorised: true };
}

cli({
    site: 'kimi',
    name: 'detail-full',
    access: 'read',
    description: 'Return complete structured Kimi chat metadata and messages over the page session.',
    domain: 'kimi.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [
        { name: 'id', positional: true, required: true, help: 'Chat ID or full /chat/<id> URL' },
    ],
    columns: [
        'ChatId',
        'Chat',
        'Messages',
        'MessageCount',
        'ReferenceKeyCount',
        'AttachmentKeyCount',
        'Complete',
        'MessagePages',
        'Termination',
        'ResponseBytes',
    ],
    func: async (page, kwargs) => {
        const chatId = parseChatId(kwargs.id);
        if (!chatId) throw new ArgumentError('id', 'must be a Kimi chat ID or URL');
        const result = await readConversation(page, chatId);
        if (result.authMissing) {
            throw new CommandExecutionError('Kimi session token is unavailable in the page', chatId);
        }
        if (result.removed) {
            throw new CommandExecutionError('GetChat returned HTTP 404', chatId);
        }
        if (result.unauthorised) {
            throw new CommandExecutionError('Kimi session expired (HTTP 401)', chatId);
        }
        if (result.error) {
            throw new CommandExecutionError(result.error, `${chatId} HTTP ${result.status || 'unknown'}`);
        }
        const chat = result.chat;
        if (!chat || chat.id !== chatId) {
            throw new CommandExecutionError('GetChat returned a different or missing chat', chatId);
        }
        if (!Array.isArray(result.messages)) {
            throw new CommandExecutionError('ListMessages did not return a messages array', chatId);
        }
        const lastPage = result.pages[result.pages.length - 1] || { count: 0, response: {} };
        const responseBytes = result.pages.reduce(
            (total, entry) => total + JSON.stringify(entry.response).length,
            result.chatBytes,
        );
        const referenceKeys = result.pages.reduce(
            (total, entry) => total + countMatchingKeys(entry.response, (key) => key.includes('ref') || key.includes('citation')),
            0,
        );
        const attachmentKeys = result.pages.reduce(
            (total, entry) => total + countMatchingKeys(entry.response, (key) => key.includes('attach') || key.includes('file') || key.includes('media')),
            0,
        );
        return [{
            ChatId: chatId,
            Chat: chat,
            Messages: result.messages,
            MessageCount: result.messages.length,
            ReferenceKeyCount: referenceKeys,
            AttachmentKeyCount: attachmentKeys,
            Complete: result.exhausted,
            MessagePages: result.pages.length,
            Termination: result.exhausted
                ? (lastPage.count === 0 ? 'empty-final-page' : 'no-token')
                : 'page-limit',
            ResponseBytes: responseBytes,
        }];
    },
});
