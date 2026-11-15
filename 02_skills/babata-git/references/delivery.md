# Worktree、PR 与收尾

## Worktree

先 git worktree list --porcelain，核基线 SHA／时间及目标路径；要求最新时先 fetch 确认成功，
不以旧 tracking ref 冒称最新，不静默改全局 Git／SSH 配置。新分支命名遵守仓库。
linked worktree 的 .git 是文件，不能据此判非仓库。HEAD/index 独立，refs 和对象共享；
不 checkout 另一工作树占用的分支，不假设路径前缀代表所有权，不清理别人的工作树。
git -C 可以精确操作，但 main 工作树 dirty 时仍需保护。

## Commit／PR

检查 unstaged／staged 真实差异，在批准范围逐路径 add；untracked 先核内容来源。shell 用
参数数组／字面路径，不拼接未信任输入。读差异注意敏感内容，公开回复只列必要脱敏摘要。
检查测试、许可证、生成物和秘密；预检不替代评审。无项目规定时偏好单一意图小提交／短分支
和 PR；不默认文档可直推主干。提交格式、签名、署名、审查和 merge 均从项目，不去掉用户署名。
Issue 由产品流程按独立结果／依赖拆分，不按文件数机械拆分。

用现成 forge CLI／API，先确认仓库身份，不输出 token。响应不明先查 remote ref／已有 PR；
本地 exit 0 或建好 PR 不证明 CI 过或 merge 成。

## Merge／清理／回退

merge 需请求包含该动作，目标、head 和 gate 均正确；不绕过保护、关检查或擅开 auto-merge。
清理需授权、精确身份和归属，查 dirty／untracked、未推送提交、关联 PR 和 worktree lock。
squash 后祖先关系可能不同，查实际 PR merge 记录，不因 branch --merged 没列就直接强删。
未授权或归属不清保留，主工作树／宿主管理工作树不自行移除。
共享历史回退通常用 revert；rewrite／force push 仅在明确允许且影响确认后，使用精确预期旧
OID 的 lease；--force-with-lease 不是自动授权或绝对安全。

## 外部依据

读取日期 2026-11-15；是参考不是项目政策。原技能经验重新表述，未复制脚本。

- [GitHub Flow](https://docs.github.com/en/get-started/using-github/github-flow)：轻量分支／PR。
- [Git worktree](https://git-scm.com/docs/git-worktree)：linked worktree 与共享 refs。
- [Superpowers](https://github.com/obra/superpowers)：借鉴隔离工作区与收尾职责分离，不引入整套框架。
