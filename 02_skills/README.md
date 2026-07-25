# Skills

```text
02_skills/
├── 00_specs/                 # 目标能力规格（可早于实现）
├── babata-collect/           # P7 总收集 Skill：内部按来源 route/recipe 编排到统一 C0
└── babata-bailian-clean/     # P5 真实 Agent 引导 Skill：多模态清洗与百炼
```

## 规则

1. 规格可以提前存在；真实 `SKILL.md` 只在对应能力已有真实路径时创建。
2. Skill / Agent / 浏览器 / 脚本 **不是** 数据权威；最终正式入库仍走 Babata 核心。
3. 原件只读。OCR、转写、摘要、标签、结构化结果都是派生物，写入 `BABATA_DATA_HOME`，不进 Git。
4. P5 当前可用的清洗引导是 `babata-bailian-clean`；`babata process` 正式 CLI Skill 仍待 C1/provider 与 TC-03/TC-04。
5. 收集只暴露 `babata-collect` 一个 Skill；Capture/Routes 是内部能力规格，平台和真实 case
   是内部 recipe/证据，不创建一来源一个 Skill。
6. `babata-collect` 在统一 C0 提交和回读后结束，不调用 C1；同一来源新增形态时扩展 recipe、
   capability 和测试。

安装到本机 Agent skills 目录（可选）：

```text
复制或 junction 本目录下需要启用的正式 Skill：
babata-collect       -> %USERPROFILE%\.agents\skills\babata-collect
babata-bailian-clean -> %USERPROFILE%\.agents\skills\babata-bailian-clean
```
