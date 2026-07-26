const CONVERSATION_ID_RE = /^\d{8,}$/;

function conversationRow(cell, pageIndex, requestCursor, recent) {
    const conversation = cell?.conversation || {};
    const cellId = String(cell?.id || '');
    const conversationId = String(conversation.conversation_id || '');
    if (!CONVERSATION_ID_RE.test(conversationId)) {
        throw new Error(`recent_conv page ${pageIndex} has an invalid conversation ID`);
    }
    if (cellId && cellId !== conversationId) {
        throw new Error(
            `recent_conv page ${pageIndex} cell ${cellId} does not match conversation ${conversationId}`,
        );
    }
    return {
        Id: conversationId,
        Title: String(conversation.name || '(untitled)'),
        CreatedAt: String(conversation.create_time || ''),
        UpdatedAt: String(conversation.update_time || ''),
        PageIndex: pageIndex,
        RequestCursor: requestCursor,
        ResponseNextCursor: String(recent.next_conv_version || ''),
        PageHasMore: recent.has_more,
        PageCellCount: recent.cells.length,
    };
}

export function parseRecentPage(body, requestCursor, pageIndex) {
    const recent = body?.pull_recent_conv_chain_downlink_body;
    if (!recent || !Array.isArray(recent.cells)) {
        throw new Error(`recent_conv page ${pageIndex} did not return cells`);
    }
    if (typeof recent.has_more !== 'boolean') {
        throw new Error(`recent_conv page ${pageIndex} did not return has_more`);
    }
    return {
        hasMore: recent.has_more,
        nextCursor: String(recent.next_conv_version || ''),
        rows: recent.cells.map((cell) => conversationRow(cell, pageIndex, requestCursor, recent)),
    };
}

function sameConversation(left, right) {
    return left.Title === right.Title
        && left.CreatedAt === right.CreatedAt
        && left.UpdatedAt === right.UpdatedAt;
}

export async function collectCompleteHistory(fetchPage, options = {}) {
    const maxPages = options.maxPages || 500;
    const excludedIds = new Set(options.excludedIds || []);
    const seenCursors = new Set(['0']);
    const conversations = new Map();
    let cursor = '0';

    for (let pageIndex = 1; pageIndex <= maxPages; pageIndex += 1) {
        const direction = cursor === '0' ? 3 : 1;
        const body = await fetchPage(cursor, direction);
        const page = parseRecentPage(body, cursor, pageIndex);

        for (const row of page.rows) {
            const existing = conversations.get(row.Id);
            if (existing && !sameConversation(existing, row)) {
                throw new Error(`recent_conv returned conflicting metadata for ${row.Id}`);
            }
            if (!existing) conversations.set(row.Id, row);
        }

        if (!page.hasMore) {
            const rows = [...conversations.values()].filter((row) => !excludedIds.has(row.Id));
            return rows.map((row) => ({
                ...row,
                Complete: true,
                TotalConversations: rows.length,
            }));
        }

        if (!page.nextCursor || page.nextCursor === '0' || page.nextCursor === cursor) {
            throw new Error(`recent_conv page ${pageIndex} did not advance its cursor`);
        }
        if (seenCursors.has(page.nextCursor)) {
            throw new Error(`recent_conv cursor loop detected at ${page.nextCursor}`);
        }
        seenCursors.add(page.nextCursor);
        cursor = page.nextCursor;
    }

    throw new Error(`recent_conv exceeded the ${maxPages}-page safety limit`);
}

export function historyCompletePageFunction(endpoint, cursor, direction) {
    return `(async () => {
      const response = await fetch(${JSON.stringify(endpoint)}, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        credentials: 'include',
        body: JSON.stringify({
          cmd: 3200,
          uplink_body: {
            pull_recent_conv_chain_uplink_body: {
              limit: 20,
              message_count_per_conv: 10,
              api_version: 1,
              conv_version: Number(${JSON.stringify(cursor)}),
              direction: ${direction},
              option: {
                not_need_message: true,
                need_complete_conversation: true,
                need_coco_conversation: ${cursor === '0'},
                need_coco_bot: ${cursor === '0'},
                need_pc_pin_chain: true,
                pc_pin_query_type: 0,
              },
            },
          },
          sequence_id: globalThis.crypto?.randomUUID?.() || String(Date.now()),
          channel: 2,
          version: '1',
        }),
      });
      return { status: response.status, body: await response.text() };
    })()`;
}
