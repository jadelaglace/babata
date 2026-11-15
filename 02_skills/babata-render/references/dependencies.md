# 外部依赖与配置

本 Skill 自带全部 Babata 脚本（`scripts/`）。下列外部工具是依赖，不随 Skill 分发，经环境变量接入；都可不设，未配置的能力报告 unavailable，不静默降级。

| 能力 | 变量 | 作用 | 默认 |
| --- | --- | --- | --- |
| PNG 渲染 | `BABATA_NODE_MODULES` | 已装 marked、playwright 的 node_modules | `scripts/node_modules`（在 `scripts/` 内 `npm ci --ignore-scripts`） |
| PNG 渲染 | `BABATA_CHROMIUM_EXECUTABLE` | 兼容 Chromium 可执行文件 | Playwright 自带浏览器 |
| 正式 PNG（Rust） | `BABATA_PNG_RENDERER`、`BABATA_NODE` | 正式 outputs 使用的 render.mjs 与 Node | 未配置则正式 PNG 不可构建 |
| 复杂抠图 | `BABATA_REMBG_PYTHON` | 装有 rembg 的 Python 解释器 | 无，未配置即 blocked |
| 复杂抠图 | `BABATA_REMBG_MODEL` | rembg 模型名 | `isnet-general-use` |
| 复杂抠图 | `U2NET_HOME` | rembg 权重目录（rembg 自己读取） | `~/.u2net` |
| 语义视觉 | `BABATA_BL`、`BABATA_VISION_MODEL` | 百炼 `bl` 路径与默认模型（加工后检查时用） | PATH 上的 `bl`、`qwen3-vl-plus` |
| ComfyUI | `COMFYUI_URL` | 仅探测报告，不扫网 | 无 |

`python scripts/vision_backend.py probe` 报告各后端是否安装、配置，不花调用。凭据由各工具自己的配置解析，本 Skill 不读取、不写入日志或结果。
