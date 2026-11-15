# 有证据的诊断

## 最小分层

确认故障时间、请求路径和版本，按 client → DNS/TLS → proxy → listener → app → dependency
对齐同一请求证据。先窄只读探测和必要日志，不把 probe 永远先于日志当死规则。

- 502 的快／慢只说明时序，可能拒连、TLS、upstream reset、超时、代理策略或应用返回。
  对齐 access/error 日志、request ID、已授权 upstream 直连对照，不直接判队列／drain。
- 多个 Set-Cookie 合法，不可逗号合并。其他重复字段按其定义判断，重复不证明合并错误。
  JSON 5xx 可能来自网关，不自动说明应用收到请求。
- text/* 缺 charset 不必然乱码，先核具体媒体类型、实际字节和客户端规则。

## 端口和进程

Linux 查 ss -ltnp、PID/cgroup、绑定地址和单元；Windows 查 Get-NetTCPConnection／
拥有进程。区分 LISTEN、TIME_WAIT、复用、IPv4/6、socket activation 和权限问题；
TIME_WAIT 不是全部 Address-in-use 原因，非拥有者 PID 不动。
用实际 manager（systemd、容器或 Windows Service）控制精确服务；先查配置依赖。避免
泛化 pkill -f／killall，可能命中同机服务或 SSH 自己。优雅停止要有超时和未完成写入处置；
强杀需明确必要性与授权，重复 kill 不是诊断。

## 代理与流

对照直连／代理路径的状态和分段时间。SSE 查事件格式／flush、buffering/compression、
idle/read timeout、心跳及取消；按具体代理最小修改，不全局关缓冲。缺路由回 404/410
还是服务异常 502/503 由 API 合同决定，不统一改成代理本地 404。

## GPU／LLM

核现有进程、显存、架构、精度／量化、batch、context、KV cache、并发与运行时开销，
再做允许的小范围测量。7B／8GB／4k 不是通用结论，不为腾显存杀别人的任务。
请求超时、队列、取消、OOM 恢复按服务合同，不凭参数表冒称负载验证。

## Windows → SSH

用已有 SSH host 和主机密钥校验，不关闭 StrictHostKeyChecking，不提交私钥。
PowerShell 与远端 shell 引用层分开，复杂命令用受控脚本／原生参数，不拼未信任输入。
核原生命令 exit code 和远端状态，不只看 $?，也不把 stderr 输出当失败。
连接中断不等于命令没执行，先查版本／服务／变更结果，避免重复变更。
