---
name: babata-render
description: Babata 画面制作：把已有 Markdown、HTML 或正式输出渲染成 PNG 长图／显式结构分页；对静态图片抠图／去背景（白底、绿幕、复杂背景）、提取阴影、透明图层合成、保透明缩放。原件只读、产物写新文件。不搜索调研、不重写事实、不负责看图判断（交给 babata-vision）、不发布。
metadata:
  version: "1.2.0"
compatibility: Requires uv for Python helpers and Node.js/npm for PNG rendering; optional bl on PATH.
---

# Babata 画面制作

两类制作：已有内容转 PNG，和静态图片素材加工。都只产出新文件和可回读证据，不改原件、不写正式库。
"测量／检查透明边、杂点、裁切"属于 babata-vision；需要改素材时回到本 Skill 显式加工。搜索调研走 babata-research。

本 Skill 自带脚本，在本目录 `scripts/`（路径相对本 SKILL.md）；切出仓库即可按标准包管理运行，不找 Babata 仓库。Python 用 `uv run --script` 内联依赖；PNG 首次在 `scripts/` 执行 `npm ci --ignore-scripts`。外部工具从 PATH 和各自默认配置／缓存发现，见 [依赖与默认位置](references/dependencies.md)。缺依赖明确报告 unavailable，不设置用户环境变量、不借别的仓库脚本。

## 内容转 PNG

调用 [PNG 运行说明](references/run.md)，不要每次重写截图脚本。

- 临时内容：Markdown／HTML 和所需本地图片字体组成受控 bundle，输出到 Git 外 Assistant 工作区。long 生成完整长图；paged 只接受已按语义组织的顶层 .page 元素，不机械裁图。调整布局不能改写事实或省略内容。
- 正式知识：通过 babata outputs build --kind png 传 record/sublibrary scope；status/verify/delete/rebuild 仍走 Rust 输出服务。首个正式 profile 是 long，临时分页不冒充正式多页 output。
- 默认阅读 profile 为 1120px 宽、20px 正文。远程资源按授权先取得成本地资产，不偷偷放开网络；不带登录态，不执行脚本；缺图和溢出明确失败。
- 回读实际 PNG：中文缺字、密集表格、页界、截断、空白带和字号。长图小字要用 babata-vision 的 tiles 原分辨率分片看，不以缩略图冒充全文已查。图表曲线须落在坐标范围内、单位不混用；事实问题交回内容阶段，不由 renderer 猜改。

## 静态素材加工

调用 [素材加工说明](references/assets.md)。按材质选路径，不按省事选：

| 素材 | 路径 |
| --- | --- |
| 干净白底／棚拍 | `key-white`（默认只去掉与边框相连的白，白色主体内部保留） |
| 绿幕／蓝幕 | `key-chroma` 带 despill |
| 细发、玻璃、复杂背景 | 真实复杂后端 `vision_backend.py matte`，可再 `refine` 精修边缘带；未配置则 **blocked**，不能用 keying 顶替。当前 rembg 是分割模型，远离主体的细发会丢失，须如实说明 |
| 阴影被并进背景层 | `shadow-extract` 后 `over` 合成 |
| 已有 RGBA 图层 | `over`（第一张在最上）、`resize`（预乘缩放，不出色边） |

- 尺寸不一致默认拒绝；用户明确要求时才 `--fit stretch|contain`，并在交付中说明变换。
- 不默认 keep-largest：合法多主体、字体点号、阴影都可能是多个连通域。只有用户明确要删杂块才用。
- 白色主体、半透明玻璃和阴影是 keying 的已知弱项，结果里的 `enclosed-near-white-kept`、`semi-transparent-excess` 等发现要如实转述。
- EXIF 方向默认应用并记录；P 模式透明、16-bit 先探测再显式转换，不静默抹 alpha。

## 交付

交付产物路径、hash、参数、实际后端／模型身份、耗时和测量；交付前用 babata-vision 至少看一次多背景对照（multibg），说明实际看了哪些背景和局部。
同 run 重放先核 hash；内容／参数变更用新目录。失败不覆盖旧好产物，不删无关文件。制作结果不自动发送、发布或登记为正式知识。
