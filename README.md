
# Mode

Stand Mode

Mode 0: 不需要虚拟桌面
    OLED - Windows Desktop
    EINK - Wallpaper Mode       壁纸模式

开盖自动进入 Mode 0

Mode 1: 原生桌面模式，需要虚拟桌面
    OLED - None
    EINK - Windows Desktop      Desktop Capture

通过悬浮球或者快捷键进入 Mode 2

Mode 2: 应用置顶模式，需要虚拟桌面
    OLED - None
    EINK - Launcher             App Capture
           Windows Desktop      Desktop Capture

通过悬浮球或者快捷键进入 Mode 1

如果存在 UAC 窗口，显示 UAC 窗口，否则显示置顶的应用程序

// UACPromptEvent UAC 弹出事件，Windows Desktop Layer 临时提升优先级

# Project Layout

## Directories

1. 默认 Desktop 录屏必须存在
2. 

## Apps

1. EinkService.exe      服务本体
    WmiService          WMI 管理服务、WMI 事件

2. EinkPlus.exe         主界面
3. EinkCapturer.exe     屏幕、窗口捕获器
4. EinkSettings.exe     设置程序
5. EinkCrashReport.exe  奔溃报告

## Libraries

1. Eink 横竖屏切换
2. 