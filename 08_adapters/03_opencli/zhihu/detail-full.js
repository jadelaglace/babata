import { cli, Strategy } from '@jackwener/opencli/registry';
import {
    ArgumentError,
    AuthRequiredError,
    CommandExecutionError,
    EmptyResultError,
} from '@jackwener/opencli/errors';

const ANSWER_ID_RE = /^\d+$/;

function answerId(input) {
    const value = String(input || '').trim();
    if (ANSWER_ID_RE.test(value)) return value;
    try {
        const url = new URL(value);
        if (!['www.zhihu.com', 'zhihu.com'].includes(url.hostname)) return '';
        return url.pathname.match(/\/answer\/(\d+)/)?.[1] || '';
    } catch {
        return '';
    }
}

cli({
    site: 'zhihu',
    name: 'answer-detail-full',
    access: 'read',
    description: 'Return a complete Zhihu answer with raw HTML and original inline image URLs.',
    domain: 'www.zhihu.com',
    strategy: Strategy.COOKIE,
    browser: true,
    siteSession: 'persistent',
    navigateBefore: false,
    args: [{ name: 'id', positional: true, required: true }],
    columns: [
        'Id', 'Author', 'QuestionId', 'QuestionTitle', 'Url', 'CreatedAt', 'UpdatedAt',
        'ContentText', 'ContentHtml', 'Images', 'ImageCount',
    ],
    func: async (page, kwargs) => {
        const id = answerId(kwargs.id);
        if (!id) throw new ArgumentError('id', 'must be a Zhihu answer ID or URL');
        await page.goto(`https://www.zhihu.com/answer/${id}`, { waitUntil: 'none' });
        const apiUrl = `https://www.zhihu.com/api/v4/answers/${id}?include=content,voteup_count,comment_count,author,created_time,updated_time,question`;
        const result = await page.evaluate(`(async () => {
          const response = await fetch(${JSON.stringify(apiUrl)}, { credentials: 'include' });
          if (!response.ok) return { httpError: response.status };
          const data = await response.json();
          const document = new DOMParser().parseFromString(data.content || '', 'text/html');
          const normalize = (value) => String(value || '')
            .replace(/\\u00a0/g, ' ')
            .replace(/[ \\t]+\\n/g, '\\n')
            .replace(/\\n{3,}/g, '\\n\\n')
            .trim();
          const seen = new Set();
          const images = Array.from(document.querySelectorAll('img')).flatMap((image, index) => {
            const url = image.getAttribute('data-original')
              || image.getAttribute('data-actualsrc')
              || image.getAttribute('data-src')
              || image.getAttribute('src')
              || '';
            const token = image.getAttribute('data-original-token')
              || url.match(/\\/(v2-[a-f0-9]+)_/)?.[1]
              || '';
            if (!/^https:\\/\\/(?:picx|pic1|pic2|pic3|pic4|pica)\\.zhimg\\.com\\//.test(url)
                || !token || seen.has(token)) return [];
            seen.add(token);
            return [{ Index: index, Token: token, Url: url, Alt: normalize(image.getAttribute('alt')) }];
          });
          return {
            id: String(data.id || ${JSON.stringify(id)}),
            author: data.author?.name || 'anonymous',
            questionId: String(data.question?.id || ''),
            questionTitle: data.question?.title || '',
            createdAt: data.created_time ? new Date(data.created_time * 1000).toISOString() : '',
            updatedAt: data.updated_time ? new Date(data.updated_time * 1000).toISOString() : '',
            contentText: normalize(document.body?.innerText || document.body?.textContent || ''),
            contentHtml: data.content || '',
            images,
          };
        })()`);
        if (result?.httpError === 401 || result?.httpError === 403) {
            throw new AuthRequiredError('www.zhihu.com', 'Zhihu answer requires a logged-in browser session.');
        }
        if (result?.httpError === 404) {
            throw new EmptyResultError('zhihu answer-detail-full', `Answer ${id} was not found.`);
        }
        if (!result || result.httpError || !result.contentText || !result.contentHtml) {
            throw new CommandExecutionError(
                `Zhihu answer ${id} returned an incomplete payload`,
                'Retry after confirming the answer is reachable in Chrome.',
            );
        }
        return [{
            Id: result.id,
            Author: result.author,
            QuestionId: result.questionId,
            QuestionTitle: result.questionTitle,
            Url: result.questionId
                ? `https://www.zhihu.com/question/${result.questionId}/answer/${id}`
                : `https://www.zhihu.com/answer/${id}`,
            CreatedAt: result.createdAt,
            UpdatedAt: result.updatedAt,
            ContentText: result.contentText,
            ContentHtml: result.contentHtml,
            Images: result.images,
            ImageCount: result.images.length,
        }];
    },
});
