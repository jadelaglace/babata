# Skills

```text
02_skills/
├── 00_specs/                 # 目标能力规格（可早于实现）
└── babata-bailian-clean/     # P5 真实 Agent 引导 Skill：多模态清洗与百炼
```

## 规则

1. 规格可以提前存在；真实 `SKILL.md` 只在对应能力已有真实路径时创建。
2. Skill / Agent / 浏览器 / 脚本 **不是** 数据权威；最终正式入库仍走 Babata 核心。
3. 原件只读。OCR、转写、摘要、标签、结构化结果都是派生物，写入 `BABATA_DATA_HOME`，不进 Git。
4. P5 当前可用的清洗引导是 `babata-bailian-clean`；`babata process` 正式 CLI Skill 仍待 C1/provider 与 TC-03/TC-04。

安装到本机 Agent skills 目录（可选）：

```text
复制或 junction 本目录下的 babata-bailian-clean
到 %USERPROFILE%\.agents\skills\babata-bailian-clean
```
