# 外部依据与取舍

读取日期 2026-11-15；是方法／协议参考，不是部署授权或目标机事实。原技能经验重新核证，
探针独立窄实现，不引入源目录依赖。

- [RFC 9110 §5.3](https://www.rfc-editor.org/rfc/rfc9110.html#section-5.3)：重复字段按定义处理，
  Set-Cookie 不简单逗号合并，不将重复本身判故障。
- [Python HTTPResponse](https://docs.python.org/3/library/http.client.html)：getheaders 保留
  名称／值列表；本探针不读 body。
- [systemd.service](https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html)：
  类型、生命周期、停止超时匹配实际进程，不通用化 kill/nohup。
- [Ansible check/diff](https://docs.ansible.com/projects/ansible/latest/playbook_guide/playbooks_checkmode.html)：
  支持范围和泄密风险，不将 check 通过当部署成功。
- [Google SRE canary](https://sre.google/workbook/canarying-releases)：可复现发布、有限影响、
  观察和回滚，按单机／服务规模使用，不引入大厂平台。
