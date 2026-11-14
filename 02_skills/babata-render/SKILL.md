---
name: babata-render
description: 将已有 Markdown、HTML 或 Babata 正式输出渲染成 PNG 长图／显式结构分页，核对完整性与视觉质量；不搜索调研、不重写事实、不发布。
metadata:
  version: "1.0.1"
---

# Babata PNG 渲染

已有内容的一种输出模态。研究不是前置条件，输入不必先进入知识宇宙。HTML 是当前渲染中间格式，不是知识本体。调用 [运行说明](references/run.md)，不要每次重写截图脚本。

脚本根优先用本地 BABATA_REPO_HOME；否则定位当前 Babata checkout，核对 05_scripts/atoms/render.mjs 后运行。Skill 安装目录不等于仓库根；找不到实现或依赖就报告 unavailable，不拿别的文件冒充本原子输出。

## 独立调用

- 临时内容：Markdown／HTML 和所需本地图片字体组成受控 bundle，输出到 Git 外 Assistant 工作区。long 生成完整长图；paged 只接受已按语义组织的顶层 .page 元素，不机械裁图。调整布局不能改写事实或省略内容。
- 正式知识：通过 babata outputs build --kind png 传 record/sublibrary scope；status/verify/delete/rebuild 仍走 Rust 输出服务。首个正式 profile 是 long，临时分页不冒充正式多页 output。显示范围沿用现有 record 投影，不声称含尚未取得的全文／附件。

## 执行与质量

先看内容身份和资产再选模式。默认阅读 profile 为 1120px 宽、20px 正文、清楚层级与对比，可调宽度。远程资源按授权先取得成本地资产，不偷偷放开网络。不带登录态，不执行脚本；危险／不支持 HTML、缺图和溢出明确失败。不把此约束说成强 OS 沙箱。

回读实际 PNG，查看中文缺字、字体、密集表格、页界、标题、截断、空白带和字号；机械 pass 不等于视觉 pass，只报告已检查范围。保留图表单位、完整数据范围和证据，不能用成功截图掩盖内容损失。

含图表时核对所有可见曲线、阈值和标注是否落在坐标范围内；不同单位、基准和日期不得混用。局部放大须保留可直接查看的完整范围。参数化图表检查边界值、无事件和叠加状态；发现事实或计算问题交回内容阶段，不由 renderer 猜改，未解决缺陷随交付说明。

同 run 重放先核 hash；内容／资产／参数变更用新目录。正式显式 rebuild 创建新 generation。失败不退出旧好产物，不删无关文件。异常 lock／staging 先核真实进程及文件，不在活跃写入时清理，不把人工对账包装成自动恢复。

交付图片路径、模式／尺寸及限制，研究核证与渲染结果分开。渲染不自动发送或发布。
