import { cli, Strategy } from '@jackwener/opencli/registry';
import {
    ArgumentError,
    CommandExecutionError,
    EmptyResultError,
} from '@jackwener/opencli/errors';

const KIMI_URL = 'https://www.kimi.com/';
const CHAT_ID_RE = /^[0-9a-f-]{8,}$/i;
const CAPTURE_PATTERN = 'ChatService/GetChat|ChatService/ListMessages';

function parseChatId(input) {
    const value = String(input || '').trim();
    if (CHAT_ID_RE.test(value)) return value.toLowerCase();
    try {
        const url = new URL(value, KIMI_URL);
        if (url.protocol !== 'https:' || url.hostname !== 'www.kimi.com') return '';
        return url.pathname.match(/^\/chat\/([0-9a-f-]{8,})$/i)?.[1]?.toLowerCase() || '';
    } catch {
        return '';
    }
}

function responseJson(entry, operation) {
    if (!entry) throw new EmptyResultError('kimi detail-full', `${operation} was not captured.`);
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
    try {
        return JSON.parse(String(entry.responsePreview || ''));
    } catch (error) {
        throw new CommandExecutionError(
            `${operation} returned invalid JSON`,
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

async function captureConversation(page, chatId) {
    if (typeof page.startNetworkCapture !== 'function' || typeof page.readNetworkCapture !== 'function') {
        throw new CommandExecutionError(
            'OpenCLI Browser Bridge network capture is unavailable',
            'Install or update the OpenCLI Chrome extension.',
        );
    }
    await page.goto(`${KIMI_URL}chat/history`, { waitUntil: 'none' });
    await page.wait(0.5);
    const started = await page.startNetworkCapture(CAPTURE_PATTERN);
    if (!started) {
        throw new CommandExecutionError(
            'OpenCLI Browser Bridge network capture is unavailable',
            'Install or update the OpenCLI Chrome extension.',
        );
    }
    await page.goto(`${KIMI_URL}chat/${chatId}?chat_enter_method=history`, { waitUntil: 'none' });
    for (let attempt = 0; attempt < 30; attempt++) {
        const mounted = await page.evaluate(`(() => {
          const list = document.querySelector('.chat-content-list') || document.querySelector('.message-list');
          return !!list && list.querySelectorAll('.chat-content-item, .segment').length > 0;
        })()`);
        if (mounted) break;
        await page.wait(0.5);
    }
    // LoadingFinished must run before capture-read, which drains the bridge buffer.
    await page.wait(1);
    return await page.readNetworkCapture();
}

cli({
    site: 'kimi',
    name: 'detail-full',
    access: 'read',
    description: 'Return complete structured Kimi chat metadata and messages from the page network responses.',
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
        'ResponseBytes',
    ],
    func: async (page, kwargs) => {
        const chatId = parseChatId(kwargs.id);
        if (!chatId) throw new ArgumentError('id', 'must be a Kimi chat ID or URL');
        const entries = await captureConversation(page, chatId);
        const getChatEntry = entries.find((entry) => String(entry?.url || '').includes('ChatService/GetChat'));
        const listMessagesEntry = entries.find((entry) => String(entry?.url || '').includes('ChatService/ListMessages'));
        const chatResponse = responseJson(getChatEntry, 'GetChat');
        const messagesResponse = responseJson(listMessagesEntry, 'ListMessages');
        const chat = chatResponse?.chat;
        const messages = messagesResponse?.messages;
        if (!chat || chat.id !== chatId) {
            throw new CommandExecutionError('GetChat returned a different or missing chat', chatId);
        }
        if (!Array.isArray(messages)) {
            throw new CommandExecutionError('ListMessages did not return a messages array', chatId);
        }
        const nextPageToken = messagesResponse.nextPageToken || messagesResponse.next_page_token || '';
        return [{
            ChatId: chatId,
            Chat: chat,
            Messages: messages,
            MessageCount: messages.length,
            ReferenceKeyCount: countMatchingKeys(messagesResponse, (key) => key.includes('ref') || key.includes('citation')),
            AttachmentKeyCount: countMatchingKeys(messagesResponse, (key) => key.includes('attach') || key.includes('file') || key.includes('media')),
            Complete: !nextPageToken,
            ResponseBytes: JSON.stringify(chatResponse).length + JSON.stringify(messagesResponse).length,
        }];
    },
});
