# 可恢复研究 run

在已定位 Babata 仓库根运行 Node helper。所有输入和 run 在 Git 外，不能放凭据或 cookie。helper 不联网；真实搜索与读取由宿主执行。

    node 05_scripts/atoms/research.mjs init --request <absolute-request.json> --run <absolute-new-run-dir>
    node 05_scripts/atoms/research.mjs record --event <absolute-event.json> --run <run-dir>
    node 05_scripts/atoms/research.mjs finish --conclusion <absolute-conclusion.json> --run <run-dir>
    node 05_scripts/atoms/research.mjs status --run <run-dir>
    node 05_scripts/atoms/research.mjs verify --run <run-dir>

- request：question、scope、producer 字符串，depth 为 sources 或 research；可附预算、日期和工具约束，全部纳入请求身份。
- event：id（字母数字下划线横线）、kind、provider、实际 observed_at、status（succeeded/failed/inaccessible），失败须 reason。
- search event：kind:search、实际 query、results:[{url,title,...}]；可加参数与摘要。
- body event：kind:body、url、coverage:full|relevant-excerpt|none、实际 body。不可读用 none＋reason，不能用摘要冒充正文。
- conclusion：markdown、producer、limitations:[]、claims:[{text,kind,sources:[]}]。kind 为 evidence/inference/assumption/recommendation/unresolved；前两类引用已读正文 event ID。research 深度至少有正文和主张，sources 深度不强制研究结论。

同 ID 同字节回放成功，异内容拒绝。finalized 不再追加，新观察用新 run。report.md 可独立传给 PNG 原子，不需要入 C0。verify 是完整性核查，不是事实认证。

异常时先核进程与文件 hash。相同 event 重交可接纳同字节 orphan，冲突失败；初始化中断或 renderer staging 不明时先对账，必要时新建目录，不把新目录称作原 run 自动恢复。
