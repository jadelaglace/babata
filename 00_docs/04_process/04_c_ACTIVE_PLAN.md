# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

#248 调研／候选方案已交付 PR #249；本轮仅定义终态，不是金融功能可用或投资效果验收。
完整方案在 `proposals/2026-11-15-finance-source-signals/README.md`，原话与研究证据在
`D:/BabataEvidence/finance-stk02-discovery-20261115/`，当前交付状态由 DOC-USAGE 拥有。
2026-11-17 后续已建立 Git 外文件骨架，工作入口为 `D:/BabataAssistant/solutions/finance-stk02/README.md`，
用户逐条补充即可，不要求一次填完；对应结构为同提案 `WORKSPACE.md`。
本轮原话／骨架验证／终态在 `D:/BabataEvidence/finance-stk02-scaffold-20261117/`；验证结果查 DELIVERY.json。
文件建立不代表金融程序实施，下一次根据新输入更新对应工作文件；不自动采集、接券商、
交易、订阅、部署定时任务或恢复其他 PR／发丝队列。
本次不合并、不安装、不改 WBS／StockMesh；#246 旧工作树既有 dirty 改动保留。

## 2. 下次开工队列（禁止恢复时自动执行）

### #235 复杂发丝真正 alpha matting 后端：queued / requires-explicit-resume

- 用户原话（2026-11-15）：“先收尾，发丝的记一下，不急”。不由 Issue OPEN、空 Goal 或本条触发。
- 缺口：R04 冻结门槛为全图 MAE≤0.05 且发丝边缘带 MAE≤0.25；rembg 是分割模型，Round-1 带误差 0.436，
  加 `refine` 后新变体 0.343。需要可本地运行的 matting 模型（核许可、权重 hash、CPU 可行性），新 seed 复验。
- 证据：`D:/BabataEvidence/render-vision-235/REPORT.md` 第 3、5 节。

不得自动启动其他来源、Web 输出或通用 Obsidian capability。
