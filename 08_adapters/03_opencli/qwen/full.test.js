import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import vm from 'node:vm';

// Exercise the registered command without requiring a global OpenCLI install in CI.
const source = await readFile(new URL('./full.js', import.meta.url), 'utf8');
let command;
const registry = new vm.SyntheticModule(['cli', 'Strategy'], function () {
    this.setExport('cli', (definition) => { command = definition; });
    this.setExport('Strategy', { COOKIE: 'cookie' });
});
const errorModule = new vm.SyntheticModule(
    ['ArgumentError', 'CommandExecutionError', 'EmptyResultError'],
    function () {
        this.setExport('ArgumentError', class extends Error {});
        this.setExport('CommandExecutionError', class extends Error {});
        this.setExport('EmptyResultError', class extends Error {});
    },
);
const module = new vm.SourceTextModule(source);
await module.link((specifier) => {
    if (specifier === '@jackwener/opencli/registry') return registry;
    if (specifier === '@jackwener/opencli/errors') return errorModule;
    throw new Error(`Unexpected dependency: ${specifier}`);
});
await module.evaluate();

const id = '0123456789abcdef0123456789abcdef';
const messages = [{ session_id: id, msg_id: 'message-1', text: 'native text '.repeat(8192) }];
const body = () => ({ code: 0, data: { list: structuredClone(messages), have_next_page: false } });

function pageFor(response, { status = 200, wrap = false, title = 'Test chat' } = {}) {
    const requests = [];
    const visits = [];
    return {
        requests, visits,
        async goto(url) { visits.push(url); },
        async wait() {},
        async evaluate(script) {
            if (script === 'document.title') return wrap ? { data: title } : title;
            const value = await vm.runInNewContext(script, {
                URLSearchParams,
                document: { cookie: 'b-user-id=fixture-device' },
                fetch: async (url, options) => {
                    requests.push({ url, options });
                    return { status, text: async () => typeof response === 'string' ? response : JSON.stringify(response) };
                },
            });
            return wrap ? { data: value } : value;
        },
    };
}

test('reads the explicit session through the official GET endpoint and preserves long native rows', async () => {
    const page = pageFor(body());
    const [row] = await command.func(page, { id: id.toUpperCase() });
    assert.equal(command.access, 'read');
    assert.deepEqual(page.visits, [`https://www.qianwen.com/chat/${id}`]);
    assert.equal(page.requests.length, 1);
    const request = page.requests[0];
    const url = new URL(request.url);
    assert.equal(url.origin + url.pathname, 'https://chat2-api.qianwen.com/api/v1/session/msg/list');
    assert.equal(url.searchParams.get('session_id'), id);
    assert.equal(url.searchParams.get('page_size'), '100');
    assert.equal(request.options.method, undefined); // fetch defaults to GET.
    assert.equal(request.options.credentials, 'include');
    assert.equal(row.Complete, true);
    assert.equal(row.HaveNextPage, false);
    assert.equal(row.MessageCount, 1);
    assert.deepEqual(JSON.parse(JSON.stringify(row.Messages)), messages);
    assert.ok(row.Messages[0].text.length > 65_536);
});

test('accepts an official URL and wrapped bridge response', async () => {
    const [row] = await command.func(pageFor(body(), { wrap: true }), { id: `https://www.qianwen.com/chat/${id}` });
    assert.equal(row.SessionId, id);
    assert.equal(row.Title, 'Test chat');
});

test('rejects invalid IDs and foreign URLs before any navigation or request', async () => {
    for (const value of ['all', '', 'https://example.com/chat/' + id, 'http://www.qianwen.com/chat/' + id]) {
        const page = pageFor(body());
        await assert.rejects(command.func(page, { id: value }), /id/);
        assert.equal(page.visits.length, 0);
        assert.equal(page.requests.length, 0);
    }
});

test('rejects failed, zero and missing HTTP status even with an otherwise successful body', async () => {
    for (const status of [503, 0, null, '']) {
        await assert.rejects(command.func(pageFor(body(), { status }), { id }), /HTTP/);
    }
    const page = pageFor(body());
    page.evaluate = async () => ({ body: JSON.stringify(body()) });
    await assert.rejects(command.func(page, { id }), /HTTP/);
});

test('rejects invalid JSON and application failures', async () => {
    await assert.rejects(command.func(pageFor('<html>login</html>'), { id }), /not valid JSON/);
    await assert.rejects(command.func(pageFor({ code: 401, message: 'Login required' }), { id }), /code 401/);
});

test('rejects incomplete pagination, missing termination and empty or foreign message chains', async () => {
    for (const [change, pattern] of [
        [(b) => { b.data.have_next_page = true; }, /paginated/],
        [(b) => { delete b.data.have_next_page; }, /omitted have_next_page/],
        [(b) => { b.data.have_next_page = 'false'; }, /omitted have_next_page/],
        [(b) => { b.data.list = []; }, /qianwen detail-full/],
        [(b) => { b.data.list[0].session_id = 'foreign'; }, /different session/],
        [(b) => { b.data.list = null; }, /did not match/],
    ]) {
        const response = body();
        change(response);
        await assert.rejects(command.func(pageFor(response), { id }), pattern);
    }
});
