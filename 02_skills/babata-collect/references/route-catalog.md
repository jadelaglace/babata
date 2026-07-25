# Route Catalog

## Runtime Rule

Run `babata --json capabilities list` before collection. The runtime descriptor overrides this
snapshot. Treat repository documentation status `available` as runtime `enabled`. Stop on
`disabled`, `unavailable`, a missing descriptor, or an unknown source.

The authoritative research table is
`00_docs/03_architecture/08_SOURCE_TOOL_RESEARCH.md` when working in the Babata repository.

## Current User Routes

| Route | Source | Current recipe status | Load |
| --- | --- | --- | --- |
| `source.onenote` | OneNote official PDF/MHT exports | enabled | `source-onenote.md` |
| `source.evernote` | Evernote / 印象笔记 `.notes` export | enabled | `source-evernote.md` |
| `source.doubao` | 豆包 conversations and attachments | disabled | `source-doubao.md` |
| `source.feishu` | 飞书 Docs/Wiki/knowledge | disabled | `source-browser-and-ui.md` |
| `source.yuque` | 语雀 | disabled | `source-browser-and-ui.md` |
| `source.wechat_favorites` | 微信收藏 | disabled | `source-browser-and-ui.md` |
| `source.wechat_articles` | 微信公众号文章 | disabled | `source-browser-and-ui.md` |
| `source.wechat_channels` | 微信视频号 | disabled by user decision | `source-browser-and-ui.md` |
| `source.wechat_chats` | 微信聊天 | disabled | `source-browser-and-ui.md` |
| `source.zhihu` | 知乎收藏/内容 | disabled | `source-browser-and-ui.md` |
| `source.bilibili` | Bilibili 收藏/媒体 | disabled | `source-browser-and-ui.md` |
| `source.xiaohongshu` | 小红书收藏 | disabled | `source-browser-and-ui.md` |
| `source.douyin` | 抖音收藏 | disabled by user decision | `source-browser-and-ui.md` |
| `source.browser_bookmarks` | browser bookmarks | disabled and last priority | `source-browser-and-ui.md` |
| `source.browser_pages` | current pages/selections | disabled | `source-browser-and-ui.md` |
| `source.kimi` | Kimi conversations | disabled | `source-browser-and-ui.md` |
| `source.chatgpt` | ChatGPT conversations | disabled | `source-browser-and-ui.md` |
| `source.local_files` | local files/directories | disabled as a daily route | `source-browser-and-ui.md` |
| `source.first_party` | user-authored content | use Workspace semantics, not collection | `source-browser-and-ui.md` |

An adapter class existing in source code does not enable a route. Likewise, an E3 case proves only
its tested shape until the capability registry exposes the route as enabled.

## Tool Selection Order

Use the first complete, lawful option:

1. official free bulk migration or export;
2. existing maintained plugin or script export;
3. Agent-led low-touch export;
4. small development for a repeatable bulk route;
5. paid/VIP capability after explicit user decision;
6. heavy development or complex tool flow;
7. continuous human/Agent collaboration;
8. manual-only fallback.

Browser and desktop control are execution tools, never generic replacements for platform routes.
When a platform is unknown, first investigate its official export/API/CLI and mature Agent tooling,
then open a normal Babata Issue to add evidence and a recipe. Do not silently use
`source.browser_pages` to claim platform support.
