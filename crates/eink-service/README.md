# Eink Service 核心服务

set_topmost_window

tcon_xx


Launcher
    |       --->   Reader -> HWND
    |                        ----> set_topmost_window(HWND)


## 启动参数

// 服务管理器无参启动，system 系统权限，Up-Half
eink-service

// BottomHalf 模式启动，admin 管理员权限
eink-service /bottom-half

eink_service::bottomhalf::
eink_service::windows_services::
