# 媒体路由与本地门槛

## 1. 盘点

对目标根目录：

- 按扩展名分组计数与总字节
- 每类取：用户点名样本，否则偏小样本
- 标出异常：>200MB 视频、超长竖图、扫描版 PDF、加密文档

## 2. 探针命令（按需）

```bash
ffprobe -v error -show_entries format=duration,size -show_entries stream=codec_type,codec_name,width,height -of default=noprint_wrappers=1 <file>
```

Python 思路：

- 图：`PIL.Image.open` → size/mode
- PDF：`pypdf` 抽字；`pymupdf` 渲页
- DOCX：`python-docx` 段落+表
- XLSX：`openpyxl` 前 30 行 × 12 列预览
- PPTX：`python-pptx` 或 zip 内 `ppt/slides/slide*.xml` 的 `<a:t>`

## 3. 规范化建议参数

| 输入 | 建议 |
|------|------|
| 图送 VL | 最长边 ≤ 2048；PNG/JPEG；去 alpha |
| PDF 页图 | 2x render，再视情况缩到 2048 |
| ASR 音频 | mono, 16 kHz, pcm_s16le wav；试跑 60–180s |
| 视频预览 | 可选 30–60s H.264 低码率；截帧 `-ss` 选有板书处 |
| 文本送 LLM | 先 4k–12k 字符；超长分段或先本地缩略 |

## 4. 路由决策树

```text
有可靠文本？
  是 → text chat 结构化
  否 → 是音视频？
          是 → 抽音频 ASR（+可选截帧）
          否 → 是图或可渲成图？
                  是 → vision OCR/描述
                  否 → 记录无法处理原因
```

## 5. 成本与范围控制

- 新目录：每类 1 样本
- 通过后再：按周次/科目分批
- 视频全长：单独确认（时间与费用）
- 同一原件已有合格派生物：默认跳过，除非用户要求重跑
