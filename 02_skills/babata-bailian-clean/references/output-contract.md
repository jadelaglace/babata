# 派生物契约

## manifest.json（建议字段）

```json
{
  "task": "omba25-week1-miniv",
  "created_at": "ISO-8601",
  "source_root": "C:/path/to/originals",
  "output_root": "C:/path/to/generated/...",
  "items": [
    {
      "id": "pdf-001",
      "source_path": "...",
      "source_sha256": "optional",
      "c0_revision_id": "rev_... or null",
      "c0_item_id": "item_... or null",
      "media_type": "pdf",
      "local_preprocess": [{"tool": "pypdf", "note": "extract text pages 1-3"}],
      "bailian": [{"cmd": "text chat", "model": "qwen..."}],
      "derivatives": ["results/02-pdf-text.md"],
      "c1_registrations": [
        {
          "run_id": "run_...",
          "derivative_id": "derivative_...",
          "pipeline_id": "agent_import",
          "kind": "summary",
          "status": "succeeded"
        }
      ],
      "status": "ok|partial|failed|staged_only",
      "error": null
    }
  ]
}
```

`status`：

- `staged_only`：仅有 generated/，尚未 `process register`
- `ok`：至少一次成功 register（或用户明确只要 staging）
- `partial` / `failed`：清洗或登记有缺口

## 每个样本至少交付

- 脱敏模型响应：`results/<id>.json`（便于复查并可登记 C1）
- 完整 provider 响应如含临时签名 URL、token 或鉴权头，只能放入明确受限的证据区；普通 `results/`、manifest、REPORT 和 C1 不保留这些字段
- 可读结果：`results/<id>.md`
- 预处理文件（若有）：`preprocessed/...`
- **有 C0 revision 时**：`babata process register` 后的 `run_id` / `derivative_id` 写入 manifest

## 正式 C1 登记

见 [c1-register.md](c1-register.md)。摘要：

```bash
babata --json process register \
  --pipeline agent_import \
  --revision rev_... \
  --kind summary \
  --provider bailian_cli \
  --input-sha256 <64hex> \
  --text-file results/<id>.md
```

## REPORT.md 大纲

1. 范围与样本表  
2. 本地规范化摘要  
3. 各类型结果要点（可引用 md）  
4. 视频 ASR 能力观察（时间戳/说话人/语言）  
5. **C1 登记表**（revision / kind / run_id / 是否成功）  
6. 失败与缺口  
7. 建议的下一步批量策略  

## 禁止

- 把 API Key 写入 REPORT 或 git  
- 把临时签名 URL、token 或鉴权头写入普通 C1、manifest 或 REPORT
- 把原件拷进 git 仓库  
- 无映射地只丢一堆匿名 `image-1.png`  
- 未 register 却写「已入库 Babata / 已进入 C1」
