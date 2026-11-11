import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import vm from 'node:vm';

const source = await readFile(new URL('./detail-full.js', import.meta.url), 'utf8');
let command;
const registry = new vm.SyntheticModule(['cli', 'Strategy'], function () {
    this.setExport('cli', (definition) => { command = definition; });
    this.setExport('Strategy', { COOKIE: 'cookie' });
});
const errors = new vm.SyntheticModule(['ArgumentError', 'CommandExecutionError'], function () {
    this.setExport('ArgumentError', class extends Error {});
    this.setExport('CommandExecutionError', class extends Error {});
});
const module = new vm.SourceTextModule(source);
await module.link((specifier) => {
    if (specifier === '@jackwener/opencli/registry') return registry;
    if (specifier === '@jackwener/opencli/errors') return errors;
    throw new Error(`Unexpected dependency: ${specifier}`);
});
await module.evaluate();

const id = '01234567-89ab-cdef-0123-456789abcdef';
const xid = '0123456789abcdefghij';
function pageFor(responses, { host = 'www.kimi.com', token = 'fixture-session' } = {}) {
    const requests = [], visits = [], waits = [];
    let offset = 0;
    return {
        requests, visits, waits,
        async goto(url) { visits.push(url); host = new URL(url).hostname; },
        async wait(seconds) { waits.push(seconds); },
        async evaluate(script) {
            return vm.runInNewContext(script, {
                location: { hostname: host },
                localStorage: { getItem: () => token },
                fetch: async (url, options) => {
                    requests.push({ url, options, body: JSON.parse(options.body) });
                    const response = typeof responses === 'function'
                        ? responses(requests.at(-1), offset++) : responses[offset++];
                    assert.ok(response, 'unexpected extra API request');
                    const { status = 200, body } = response;
                    return { status, text: async () => typeof body === 'string' ? body : JSON.stringify(body) };
                },
            });
        },
    };
}
const chat = (chatId = id) => ({ body: { chat: { id: chatId, title: 'Synthetic chat' } } });
const messages = (values, nextPageToken) => ({ body: { messages: values, ...(nextPageToken ? { nextPageToken } : {}) } });

test('reads only the explicit chat, follows tokens and preserves long structured messages', async () => {
    const first = { id: 'message-1', content: 'long native text '.repeat(8192), citations: [{ url: 'https://example.com' }] };
    const page = pageFor([chat(), messages([first], 'next'), messages([{ id: 'message-2', files: [{ name: 'file' }] }])]);
    const [row] = await command.func(page, { id: `https://www.kimi.com/chat/${id}` });
    assert.equal(command.access, 'read');
    assert.equal(command.navigateBefore, false);
    assert.equal(page.visits.length, 0);
    assert.deepEqual(page.requests.map((r) => r.url.split('/').at(-1)), ['GetChat', 'ListMessages', 'ListMessages']);
    assert.ok(page.requests.every((r) => r.body.chatId === id && r.options.method === 'POST'));
    assert.equal(page.requests[1].body.pageToken, '');
    assert.equal(page.requests[2].body.pageToken, 'next');
    assert.equal(row.Complete, true);
    assert.equal(row.Termination, 'no-token');
    assert.equal(row.MessageCount, 2);
    assert.equal(row.MessagePages, 2);
    assert.deepEqual(JSON.parse(JSON.stringify(row.Messages[0])), first);
    assert.ok(row.Messages[0].content.length > 65_536);
    assert.ok(row.ReferenceKeyCount > 0 && row.AttachmentKeyCount > 0);
    assert.ok(!JSON.stringify(row).includes('fixture-session'));
});

test('accepts legacy XID and protobuf empty final page; foreign page host navigates to official origin', async () => {
    const page = pageFor([chat(xid), messages([{ id: 'm' }], 'next'), { body: {} }], { host: 'evilkimi.com' });
    const [row] = await command.func(page, { id: xid.toUpperCase() });
    assert.deepEqual(page.visits, ['https://www.kimi.com/']);
    assert.equal(row.ChatId, xid);
    assert.equal(row.MessageCount, 1);
    assert.equal(row.Termination, 'empty-final-page');
});

test('rejects invalid or foreign references before page access', async () => {
    for (const value of ['', 'all', 'https://example.com/chat/' + id, 'http://www.kimi.com/chat/' + id]) {
        const page = pageFor([]);
        await assert.rejects(command.func(page, { id: value }), /id/);
        assert.equal(page.requests.length + page.visits.length, 0);
    }
});

test('retries HTTP 401 once, reports missing session and distinguishes HTTP 404', async () => {
    const page = pageFor([{ status: 401, body: {} }, chat(), messages([])]);
    assert.equal((await command.func(page, { id }))[0].Complete, true);
    assert.deepEqual(page.waits, [2]);
    const expired = pageFor([{ status: 401, body: {} }, { status: 401, body: {} }]);
    await assert.rejects(command.func(expired, { id }), /HTTP 401/);
    assert.equal(expired.requests.length, 2);
    await assert.rejects(command.func(pageFor([], { token: '' }), { id }), /session token/);
    await assert.rejects(command.func(pageFor([{ status: 404, body: {} }]), { id }), /HTTP 404/);
});

test('rejects HTTP failures, invalid JSON and application errors on either endpoint', async () => {
    for (const failed of [{ status: 503, body: {} }, { status: 0, body: {} }, { status: null, body: {} }, { body: '<truncated' }, { body: { error: 'denied' } }]) {
        await assert.rejects(command.func(pageFor([failed]), { id }));
        await assert.rejects(command.func(pageFor([chat(), failed]), { id }));
    }
});

test('rejects foreign chat before message fetch and malformed pages or tokens', async () => {
    const foreign = pageFor([chat(xid)]);
    await assert.rejects(command.func(foreign, { id }), /different or missing chat/);
    assert.equal(foreign.requests.length, 1);
    for (const body of [{ messages: null }, { messages: 'bad' }, [], { messages: [], nextPageToken: 7 }]) {
        await assert.rejects(command.func(pageFor([chat(), { body }]), { id }), /invalid/);
    }
    await assert.rejects(command.func(pageFor([chat(), messages([], 'same'), messages([], 'same')]), { id }), /repeated/);
});

test('page limit is explicit incomplete evidence, never a complete conversation', async () => {
    const page = pageFor((request, index) => index === 0 ? chat() : messages([{ id: `m-${index}` }], `page-${index}`));
    const [row] = await command.func(page, { id });
    assert.equal(row.Complete, false);
    assert.equal(row.Termination, 'page-limit');
    assert.equal(row.MessagePages, 50);
    assert.equal(page.requests.length, 51);
});
