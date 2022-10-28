#if 0
''' '
#endif
#ifdef __cplusplus
template <typename T>
using Box = T*;
#endif
#if 0
' '''
#endif


#include <cstdarg>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

extern "C" {

/// 设置窗口为置顶
uint32_t disable_win_key();

/// 清除所有置顶窗口
uint32_t enable_win_key();

/// 设置 Eink 刷新
uint32_t eink_refresh();

/// 设置 Eink MIPI Mode
uint32_t eink_set_mipi_mode(uint32_t mode);

/// 设置 Eink 显示关机壁纸
uint32_t eink_show_shutdown_cover();

/// 设置 Eink 关机壁纸
uint32_t eink_set_shutdown_cover(const uint16_t *path, uint32_t disp_type);

/// 设置窗口为置顶
uint32_t set_window_topmost(uint32_t hwnd);

/// 清除所有置顶窗口
uint32_t clear_all_topmost_windows(uint32_t hwnd);

} // extern "C"
