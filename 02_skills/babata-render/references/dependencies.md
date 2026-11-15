# 依赖与默认位置

Skill 自带业务脚本；解释器、第三方包和权重按各工具约定管理。不需要仓库路径，不创建技能私有依赖变量，不修改用户环境。

| 路径 | 使用方式 | 标准位置／约定 |
| --- | --- | --- |
| Python 处理 | `uv run --no-project --script scripts/image_ops.py …` | PEP 723 声明 Pillow／NumPy，环境与缓存由 uv 管理 |
| rembg 分割 | `uv run --no-project --script scripts/vision_backend.py matte …` | PATH 上 uv 调随包 worker，自动选择其支持的 Python，无需手建 venv |
| rembg 权重 | 默认 `isnet-general-use`，`--model` 改模型 | 默认 `~/.u2net`，尊重 rembg 自己的 `U2NET_HOME` |
| Node 包 | 在 `scripts/` 执行 `npm ci --ignore-scripts` | 自带 lock、标准 `scripts/node_modules`，npm 缓存由 npm 管理 |
| 浏览器 | 系统 Edge → Chrome → Playwright Chromium | channel 查系统安装；缓存 Windows `%LOCALAPPDATA%/ms-playwright`，其他系统用 Playwright 默认目录 |
| 语义模型 | PATH 上 `bl`；`--model` 覆盖默认 `qwen3-vl-plus` | 凭据由 bl 自有配置／环境解析，Skill 不读写 |

没有系统浏览器时才在 `scripts/` 执行 `npx playwright install chromium`；可显式 `--browser edge|chrome|chromium`，默认 auto。尊重 Playwright 自有的 `PLAYWRIGHT_BROWSERS_PATH` 等变量，不再包一层。运行时不静默安装 Node 包或浏览器。

uv 环境、npm 缓存、浏览器缓存与模型权重不放 Babata Data／Evidence／Recovery／Assistant 四根。弱网可预置标准目录并核验 hash，不改指某项目的 venv／node_modules。随包 scripts 和 npm 包旁依赖是标准发布目录，不是业务数据。

`vision_backend.py probe` 只查 PATH 与默认权重存在，不装包、不加载模型、不花语义调用；可用性以真实操作为准。

正式 PNG 仍可用原 Rust 的 `BABATA_PNG_RENDERER`／`BABATA_NODE` 配置；普通独立 Skill 不需要它们。
