---
name: babata-git
description: 仓库交付原子：检查 Git/worktree 状态、暂存与提交风险，按目标仓库政策执行提交、推送、PR 及获准的合并收尾。用户说提交、开 PR、开 worktree、预检或收尾时使用；不决定产品需求、Epic 拆分或部署。
metadata:
  version: "1.0.0"
  compatibility: Git on PATH; Python >=3.11 (or uv) for optional preflight; forge CLI for requested remote operations.
---

# Babata Git

把明确变更安全交付到目标仓库；入口不是仓库政策，也不暗含 merge、删除或重写历史授权。

## 先辨认目标与当前授权

- 确认仓库／worktree、当前 HEAD／branch、目标分支／remote 与用户要到哪一步。读取适用的
  AGENTS、CONTRIBUTING、现行流程及 PR 规则；仓库政策优先，缺失时采用最小可逆路线。
- 查看 staged、unstaged、untracked、冲突与已有用户改动；不要用全量 add、stash、reset 或切分支
  来整理不属于本任务的内容。Git 配置和 remote URL 可能含凭据，不能整份打印。
- 本地 commit、远端 push／PR、merge、分支／worktree 清理分别核对是否在授权里；
  清楚授权的例行动作直接做，不为每条命令增加审批。评审请求只检查，不修复／推送。

## 操作与预检

按仓库要求选择隔离工作树、提交格式和合并方式；不写死前缀、squash、AI attribution，
不按文件数强制拆 Issue。产品／Issue 范围不清时返回现有产品流程，不在此创建第二套治理。
涉及 worktree、PR 和清理时读 [delivery.md](references/delivery.md)。

可选预检（在本 Skill 目录，替换实际 repo 和已核基线）：

    python scripts/preflight.py --repo <repo> --base <verified-base-ref>
    python scripts/preflight.py --repo <repo> --scope staged

也可 uv run --script scripts/preflight.py ...。脚本只读、不 fetch；默认检查 index 中新增／
修改对象及 base..HEAD 全部提交可达对象，不只看最终 diff。默认 base 只读已记录的
origin/HEAD；没有或不正确时给明确 --base，不能以猜 main／HEAD 退化出一个绿灯。

JSON 报告含范围、HEAD/base OID、dirty/worktree 摘要、疑似秘密种类与对象位置；不输出
命中行、提交正文或 remote 凭据。退出 0=声明范围无发现，1=风险或覆盖不全，2=输入／工具
错误。大 blob、提交／对象超限、浅克隆和冲突不能静默跳过后称通过。大小阈值是筛选阈值，
可用 --max-blob-bytes 调整；扫描容量用 --scan-bytes。--branch-pattern／
--conventional-commits 仅在仓库采用相应规则时显式启用。

不扫描整个历史、所有 refs、未暂存／未跟踪文件、LFS 对象内容、submodule 内部、压缩包或
所有秘密格式。需要完整审计时使用项目既有工具；零发现不等于无秘密。发现真实凭据先停止
外发，脱敏报告，按拥有者流程撤销／轮换；删除当前文件不等于清历史，不自行重写共享历史。

## 交付与恢复

按项目 gate 验证，再精确暂存并复查 index；共享工作树先核文件所有权，不自动派 Agent。
推送／建 PR 后读回 branch SHA、PR head 和检查，旧 head 绿灯不证明新 head。响应丢失先查
现有 commit／branch／PR，避免重复创建。merge 前核授权、目标和必需检查。

报告已做到哪步、commit／PR、测试范围和风险；未 merge／部署／清理就明说。独立包不依赖
其他 Skill 或 Babata checkout；四件套可用时按需返回证据，不复制职责。
验证样例见 [scenarios.md](references/scenarios.md)。
