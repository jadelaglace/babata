# 素材加工运行入口

依赖 Python ≥3.11、Pillow、NumPy；OpenCV 不需要。所有命令输出 `babata.image-atom/v1` JSON 到 stdout，并在 `--out`（Git 外绝对路径、新目录）写产物和最后写入的 manifest.json。

    python 05_scripts/atoms/image_ops.py key-white <in> --out <run> [--level 248 --soft 28] [--connectivity border|global] [--keep-largest]
    python 05_scripts/atoms/image_ops.py key-chroma <in> --out <run> --color green|blue [--tolerance 60 --despill 0.8]
    python 05_scripts/atoms/image_ops.py shadow-extract <plate> --reference <orig> --subject <subject.png> --out <run>
    python 05_scripts/atoms/image_ops.py over <top> <bottom> [...] --out <run> [--fit none|stretch|contain]
    python 05_scripts/atoms/image_ops.py resize <in> --width W [--height H] --out <run>
    python 05_scripts/atoms/image_ops.py verify --out <run>

复杂抠图（本地 rembg，可替换）：

    BABATA_REMBG_PYTHON=<独立 venv 的 python> python 05_scripts/atoms/vision_backend.py matte --image <in> --out <run> [--model isnet-general-use|u2net|birefnet-general]

- 退出码：0 成功；2 输入无效／不支持／超限／冲突；3 完整性失败；4 后端不可用。
- over 是 straight-alpha 8-bit 浮点计算后四舍五入，第一张为顶层；无色彩管理。
- key-white 默认 `--connectivity border`：只有与图像边框连通的近白区域被去除；被主体包围的近白区记为发现。真正的环形镂空背景需显式 `global`。
- 输入限 8000 万像素、512 MB；拒绝 symlink／junction、损坏文件和多帧动画。
- 同 run 同请求：校验 manifest 和产物 hash 后复用；请求不同或目录残缺：拒绝，换新 run。
- rembg 权重首次使用时从其官方发布地址下载到用户缓存；只在用户授权该素材本地处理时调用，不上传图片。
