# Skills

```text
02_skills/
├── 00_specs/                 # 可复用能力与输出合同；不声明 runtime 状态
├── babata-collect/           # 唯一收集入口：显式范围 -> C0
├── babata-clean/             # 清洗入口：C0 -> 可追溯 C1/C1B
└── babata-wbs/               # WBS 自然触发薄入口；复用本机飞书 Skills
```

## 规则

1. `00_specs` 定义可复用合同，不复制产品需求、当前 capability、批次或完成状态。
2. Skill / Agent / 浏览器 / 脚本不是数据权威；Babata 正式身份、版本和状态由 core 写入；
   飞书 WBS 等外部主权对象仍经授权 native writer 操作，不复制本地主库。
3. `babata-collect` 是唯一用户可见收集 Skill；来源差异留在内部 recipe，不创建一来源一个 Skill。
4. `babata-collect` 在 C0 提交和回读后结束，不触发 C1；`babata-clean` 只从真实 C0 开始。
5. OCR、转写、摘要、标签和结构化结果都是可追溯派生物，不覆盖原件，也不进入 Git。
6. runtime capability 以 `babata --json capabilities list` 为准；当前使用结果只查 `DOC-USAGE`。
   WBS 薄 Skill 不是 Rust capability；其宿主发现与飞书可用性分别验证。

`babata-wbs` 依赖目标宿主 `.agents/skills` 下独立维护的 `feishu-wbs` 和 `lark-base`，按需读
`lark-shared`；不调用 Hermes 或旧名 WBS 副本，不自动同步两套业务规则。
`feishu-wbs` 供薄路由加载或显式调用；本仓只维护薄入口，不复制私有表坐标、业务字段字典或凭据。
安装时把该目录链接或复制到目标宿主的 Skill 发现目录；存在目录不等于宿主已成功加载。
