# 可恢复研究 run

本 Skill 自带 Node helper（`scripts/research.mjs`，只依赖 Node 内置模块）。从任意 cwd 用脚本绝对路径调用，不查找 Babata checkout。所有输入和 run 在 Git 外，不能放凭据或 cookie。helper 不联网；真实搜索与读取由宿主执行。

    node <absolute-skill-dir>/scripts/research.mjs init --request <absolute-request.json> --run <absolute-new-run-dir>
    node <absolute-skill-dir>/scripts/research.mjs record --event <absolute-event.json> --run <run-dir>
    node <absolute-skill-dir>/scripts/research.mjs finish --conclusion <absolute-conclusion.json> --run <run-dir>
    node <absolute-skill-dir>/scripts/research.mjs status --run <run-dir>
    node <absolute-skill-dir>/scripts/research.mjs verify --run <run-dir>

每个命令都用相同的两种路径模式之一：

- `--run <absolute-dir>`：直接选择完整 run 目录，不可与 workspace/run-id 混用。
- `--workspace <absolute-dir> --run-id <id>`：落在 `<workspace>/<id>`；显式 workspace 不依赖任何 Babata 配置。
- 只传 `--run-id <id>`：使用既有 `BABATA_ASSISTANT_HOME/solutions/research/workspaces/<id>`。没有有效的 Assistant 绝对路径配置即失败，提示显式路径，不猜盘符、不写用户环境。

ID 是调用方选定的稳定幂等键，1–80 个字母/数字/下划线/横线，首字符为字母或数字，不用 Windows 保留设备名；不自动按时间新建。工作区路径本身支持中文和空格。刷新另选新 ID。各命令返回实际 `run_directory`，可用该绝对路径跨 cwd 恢复；不把路径写入请求 hash。未知参数、无效 ID、冲突模式和真实路径位于 Git 内均拒绝；status/verify 不创建不存在的 run。manifest v1 及既有 `--run` 调用保留。

例如另一项目中：

    node <absolute-skill-dir>/scripts/research.mjs init --request <absolute-request.json> --workspace "D:/我的项目资料/快速调研" --run-id topic-v1
    node <absolute-skill-dir>/scripts/research.mjs verify --workspace "D:/我的项目资料/快速调研" --run-id topic-v1

- request：question、scope、producer 字符串，depth 为 sources 或 research；可附预算、日期和工具约束，全部纳入请求身份。
- event：id（字母数字下划线横线）、kind、provider、实际 observed_at、status（succeeded/failed/inaccessible），失败须 reason。
- search event：kind:search、实际 query、results:[{url,title,...}]；可加参数与摘要。
- body event：kind:body、url、coverage:full|relevant-excerpt|none、实际 body。不可读用 none＋reason，不能用摘要冒充正文。
- conclusion：markdown、producer、limitations:[]、claims:[{text,kind,sources:[]}]。kind 为 evidence/inference/assumption/recommendation/unresolved；前两类引用已读正文 event ID。research 深度至少有正文和主张，sources 深度不强制研究结论。

同 ID 同字节回放成功，异内容拒绝。finalized 不再追加，新观察用新 run。report.md 可独立传给 PNG 原子，不需要入 C0。verify 是完整性核查，不是事实认证。

异常时先核进程与文件 hash。相同 event 重交可接纳同字节 orphan，冲突失败；初始化中断或 renderer staging 不明时先对账，必要时新建目录，不把新目录称作原 run 自动恢复。
