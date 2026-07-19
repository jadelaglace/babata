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
      "media_type": "pdf",
      "local_preprocess": [{"tool": "pypdf", "note": "extract text pages 1-3"}],
      "bailian": [{"cmd": "text chat", "model": "qwen..."}],
      "derivatives": ["results/02-pdf-text.md"],
      "status": "ok|partial|failed",
      "error": null
    }
  ]
}
```

## 每个样本至少交付

- 原始模型响应：`results/<id>.json`（便于复查）
- 可读结果：`results/<id>.md`
- 预处理文件（若有）：`preprocessed/...`

## REPORT.md 大纲

1. 范围与样本表  
2. 本地规范化摘要  
3. 各类型结果要点（可引用 md）  
4. 视频 ASR 能力观察（时间戳/说话人/语言）  
5. 失败与缺口  
6. 建议的下一步批量策略  

## 禁止

- 把 API Key 写入 REPORT 或 git  
- 把原件拷进 git 仓库  
- 无映射地只丢一堆匿名 `image-1.png`
