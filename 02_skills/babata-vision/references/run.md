# 视觉运行入口

依赖 Python 3.13、Pillow、NumPy。输出 JSON；`--out` 是 Git 外绝对路径的新 run 目录，manifest.json 最后写入。

    python 05_scripts/atoms/image_ops.py probe <img>
    python 05_scripts/atoms/image_ops.py measure <img> --out <run> --profile cutout|multi-part|translucent|pixel-art [--expect-parts N] [--layout]
    python 05_scripts/atoms/image_ops.py diff <a> <b> --out <run> [--backgrounds white,black,checker] [--tolerance 0]
    python 05_scripts/atoms/image_ops.py multibg <img> --out <run> [--zoom x0,y0,x1,y1]
    python 05_scripts/atoms/image_ops.py tiles <img> --out <run> [--tile 1600 --overlap 160]
    python 05_scripts/atoms/image_ops.py layout <img> --out <run>
    python 05_scripts/atoms/image_ops.py annotate <img> --out <run> --box x0,y0,x1,y1,标签 [--arrow x0,y0,x1,y1,标签]
    python 05_scripts/atoms/image_ops.py verify --out <run>

语义后端（可替换）：

    python 05_scripts/atoms/vision_backend.py probe
    python 05_scripts/atoms/vision_backend.py ask --backend bl --image <img> [--image <img2>] --prompt "<问题>" --out <run> [--model qwen3-vl-plus] [--expect json]
    python 05_scripts/atoms/vision_backend.py ask --backend host --model <宿主真实模型> --answer-file <宿主回答.txt> --image <img> --prompt "<问题>" --out <run>

- `bl` 后端经百炼 CLI 调千问 VL，凭据由 bl 自己解析；多图会拼成标 A/B 的一张图再发送，`sent.png` 留存实际发送的像素。
- `host` 后端只登记宿主已经实际看图得出的回答，不能用于宿主没读过的图片。
- 结果状态：succeeded／failed（empty-content、truncated、bad-json、transport 问题）。超时只停本地等待，远端状态记 unknown，不盲目重投。
- usage 来自后端返回；费用未结算时记 unknown，不以估算冒充。
- 坐标一律左上原点、x0,y0,x1,y1 含端点。tiles 的坐标映射：分片像素 (u,v) → 原图 (box_x0+u, box_y0+v)。
