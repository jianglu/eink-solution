# Eink Service 热键守护进程

eink-service 工作在 LOCAL SYSTEM 账户下，eink-service-hotkey 由 CreateProcessAsUser 在 Desktop 下启动，接受热键信息，发送到 eink-service