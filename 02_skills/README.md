# Skills

```text
02_skills/
├── 00_specs/                 # 可复用能力与输出合同；不声明 runtime 状态
├── babata-collect/           # 唯一收集入口：显式范围 -> C0
├── babata-clean/             # 清洗入口：C0 -> 可追溯 C1/C1B
├── babata-research/          # 独立网络搜索与调研，不自动登记 C0
└── babata-render/            # 独立 PNG 输出，不研究／不改写内容
```

## 规则

1. `00_specs` 定义可复用合同，不复制产品需求、当前 capability、批次或完成状态。
2. Skill / Agent / 浏览器 / 脚本不是数据权威；正式身份、版本和状态只由 Babata core 写入。
3. `babata-collect` 是唯一用户可见收集 Skill；来源差异留在内部 recipe，不创建一来源一个 Skill。
4. `babata-collect` 在 C0 提交和回读后结束，不触发 C1；`babata-clean` 只从真实 C0 开始。
5. OCR、转写、摘要、标签和结构化结果都是可追溯派生物，不覆盖原件，也不进入 Git。
6. runtime capability 以 `babata --json capabilities list` 为准；当前使用结果只查 `DOC-USAGE`。
7. 搜索调研与 PNG 可各自使用；调研汇报只是两者的组合，非第三套知识／状态权威。
   `research.mjs` 只记录宿主实际观察；正式 PNG 通过原 outputs 服务接纳。依赖与调用见各自 reference。
