# Babata 当前执行计划与进度控制

<!-- DOC-ID: DOC-ACTIVE-PLAN -->
<!-- DOC-AUTHORITY-BOUNDARY: active-plan-progress -->

本文只保存当前活动项、受限队列和恢复所需的临时结论。稳定生命周期由
[`DOC-PROCESS`](04_a_DEVELOPMENT_PROCESS.md) 定义；产品状态、完成证据和历史
分别由 `DOC-USAGE`、运行回执与 Git 历史拥有，不进入本恢复热路径。

## 1. 当前活动项（恢复时先读，最多一个）

<!-- CURRENT-ACTIVE: none -->

当前没有活动任务。Backs R2-B 已达到 `passed_r2b_recoverable_quarantine`：2,988/2,988 个授权
occurrences、5,046,508,103 bytes 已移入独立隔离区，1,867/1,867 survivors 原位回读通过，2,952 个
目录路径保持不变，独立 verifier 通过；本轮没有永久删除。终端由
`D:\BabataData\04_runtime\staging\model-workspaces\backs-memory-archive-r2b-20260823-v1\terminal-receipt.json`
保存，不得重跑 R2-B。

## 2. 下次开工队列（禁止恢复时自动执行）

### AP-20260823-07：Backs R2-C 隔离观察与去重收口

- promotion policy：`requires-explicit-resume`；R2-B 完成不自动启动 R2-C。
- scope：以 R2-B terminal、隔离区、restore map 和保留后的 `D:\Backs` 为冻结输入，完成观察期回读、
  恢复可用性抽验和精确去重阶段收口。
- protected boundary：不永久删除隔离区；624 个 unresolved、零字节、垃圾、近似内容、截图 OCR 和年度
  归档仍不进入本阶段；在用户明确命名的有界阶段内不做文件级重复确认。
- next action：用户明确要求完成 R2-C 后开新 round；否则保持 held。
- recovery entry：上述 R2-B `terminal-receipt.json`、`verification-receipt.json`、`restore-map.jsonl` 和
  quarantine root。
