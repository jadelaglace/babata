---
name: babata-ops
description: 开发机与服务器运行维护原子：环境核查、服务排障、授权部署重启、健康检查、回滚和备份恢复。用户说开发机环境、服务器运维、502/SSE/端口排查或部署恢复时使用；不替代产品架构、代码修复或云厂商专用能力。
metadata:
  version: "1.0.0"
  compatibility: Native OS tools on PATH; SSH for authorized remote targets; Python >=3.11 (or uv) for optional HTTP probe.
---

# Babata Ops

让明确的开发环境或服务在授权范围内可解释地运行／恢复。先确认机器和服务，不先杀进程。

## 目标与权限

确认本地／远端、开发／测试／部署／生产角色、服务名／端口／进程管理器、允许动作与终端。
未知角色按含用户工作／有状态环境保护。诊断只读；明确部署／重启请求可执行对应有界步骤，
不隐含升级全机、改防火墙／SSH、轮换密钥、删数据或重写 checkout。
读取 runbook、部署合同、健康判据和维护窗口。目标机地址／账户来自用户配置或既有授权，
不打印私钥、token、整份环境或私有 inventory。已有专业云工具优先复用。

## 按任务加载

- 502／端口／SSE／GPU／SSH：读 [diagnosis.md](references/diagnosis.md)。
- 安装依赖、配置、部署／重启、回滚或备份恢复：读 [operations.md](references/operations.md)。
- 外部方法及边界来源：[sources.md](references/sources.md)。

窄 HTTP 探针（在本 Skill 目录，URL 指明确授权的健康路径）：

    python scripts/probe.py --url http://127.0.0.1:8000/health --timeout 5

也可 uv run --script scripts/probe.py ...。仅 GET／HEAD，无 body／自定义鉴权参数，
不跟 redirect、不保存 cookie、TLS 验证开启、不自动重试。JSON 分开记录 transport 和
HTTP 健康（默认 2xx）；退出 0=2xx，1=非2xx，2=输入错，4=连接／协议失败，5=超时。
重复头保留逐行结构，仅 Content-Type／Content-Length 值可见，其他值隐藏；不返回正文、
URL 路径／查询／凭据或原始异常。timeout 限 socket 等待，不是 DNS／全流程硬期限。
只测响应头，不证明业务健康／SLA，不负责慢滴流、SSE 或大正文测试。

需要鉴权时用工具原生安全配置／既有受保护探针，不把秘密塞进命令历史或私加变量。
GET／HEAD 也可能被应用错误实现为写操作，先确认是允许探测的健康端点，不遍历未知 URL。

## 证据、执行与收尾

观察（状态、时间、端口、日志）与推断、下一验证分开。响应时长、重复头、错误页格式只能
提供线索，不能直接锁定故障层。日志可能敏感，只取必要范围，公开输出前脱敏。
变更前记录目标版本／基线、影响半径、可恢复前提；执行后核进程、版本和真实请求路径。
状态未知先对账再重试；达到目标报 unchanged，不反复重启／安装，不承诺外部 exactly-once。
返回目标、已执行／未做动作、前后身份、验证、剩余故障与恢复入口。备份存在不等于可恢复，
进程起来不等于用户路径可用，未实测远端就明说。独立包无需 Babata 或四件套；现有流程
可用时按需回传证据，不另建一套。行为题见 [scenarios.md](references/scenarios.md)。
