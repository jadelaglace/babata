import assert from 'node:assert/strict';
import test from 'node:test';

import {
    collectCompleteHistory,
    historyCompletePageFunction,
    parseRecentPage,
} from './history-complete.js';

function page(cells, hasMore, nextCursor) {
    return {
        pull_recent_conv_chain_downlink_body: {
            cells: cells.map(({ id, title = id, created = '', updated = '' }) => ({
                id,
                conversation: {
                    conversation_id: id,
                    name: title,
                    create_time: created,
                    update_time: updated,
                },
            })),
            has_more: hasMore,
            next_conv_version: nextCursor,
        },
    };
}

test('collects all pages, preserves order, deduplicates, and excludes requested IDs', async () => {
    const calls = [];
    const pages = [
        page([{ id: '11111111' }, { id: '22222222' }], true, '100'),
        page([{ id: '22222222' }, { id: '33333333' }], false, ''),
    ];
    const rows = await collectCompleteHistory(async (cursor, direction) => {
        calls.push({ cursor, direction });
        return pages.shift();
    }, { excludedIds: ['22222222'] });

    assert.deepEqual(calls, [
        { cursor: '0', direction: 3 },
        { cursor: '100', direction: 1 },
    ]);
    assert.deepEqual(rows.map((row) => row.Id), ['11111111', '33333333']);
    assert.equal(rows[0].Complete, true);
    assert.equal(rows[0].TotalConversations, 2);
});

test('rejects a missing, repeated, or looping continuation cursor', async () => {
    await assert.rejects(
        collectCompleteHistory(async () => page([], true, '')),
        /did not advance/,
    );
    await assert.rejects(
        collectCompleteHistory(async (cursor) => page([], true, cursor === '0' ? '100' : '100')),
        /did not advance|cursor loop/,
    );
});

test('rejects malformed pages and mismatched cell identities', () => {
    assert.throws(() => parseRecentPage({}, '0', 1), /did not return cells/);
    assert.throws(() => parseRecentPage({
        pull_recent_conv_chain_downlink_body: {
            cells: [{ id: '11111111', conversation: { conversation_id: '22222222' } }],
            has_more: false,
        },
    }, '0', 1), /does not match/);
});

test('rejects conflicting duplicate metadata and the page safety limit', async () => {
    const pages = [
        page([{ id: '11111111', title: 'first' }], true, '100'),
        page([{ id: '11111111', title: 'changed' }], false, ''),
    ];
    await assert.rejects(collectCompleteHistory(async () => pages.shift()), /conflicting metadata/);
    await assert.rejects(
        collectCompleteHistory(async (cursor) => page([], true, String(Number(cursor) + 1)), {
            maxPages: 2,
        }),
        /safety limit/,
    );
});

test('builds first and continuation page requests with the official directions', () => {
    const first = historyCompletePageFunction('https://www.doubao.com/im/chain/recent_conv', '0', 3);
    const older = historyCompletePageFunction('https://www.doubao.com/im/chain/recent_conv', '100', 1);
    assert.match(first, /direction: 3/);
    assert.match(first, /need_coco_conversation: true/);
    assert.match(older, /direction: 1/);
    assert.match(older, /need_coco_conversation: false/);
});
