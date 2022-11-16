#ifndef __IT8951_USB_API___
#define __IT8951_USB_API___

#include "Ntddscsi.h"
#include "winioctl.h"
#include "IT8951UsbCmd.h"
#include "winusb_cmd.h"

#include <atomic>
using namespace std;
//*********************************************************
//  typedef 
//*********************************************************
#define SPT_BUF_SIZE  (60*1024)//(2048)

typedef struct
{
	SCSI_PASS_THROUGH stSPTD;
	BYTE DataBuffer[SPT_BUF_SIZE];

}SCSI_PASS_THROUGH_WITH_BUFFER;

enum DefaultImage_CheckFlag
{
	NONE = 0,
	MIPI_CHK,
	GPIO,
	BOTH
};

//*********************************************************
//  Function API
//*********************************************************
DWORD ITEReadRegAPI(DWORD ulRegAddr, DWORD* pulRegVal);
DWORD ITEWriteRegAPI(DWORD ulRegAddr, DWORD ulRegVal);
DWORD ITEInquiryAPI(BYTE* bFlag);
DWORD ITEGetSystemInfoAPI(_TRSP_SYSTEM_INFO_DATA* pstSystemInfo);
HANDLE ITEOpenDeviceAPI(char* pString);
void ITECloseDeviceAPI(void);
DWORD ITEReadMemAPI(DWORD ulMemAddr, WORD usLength, BYTE* RecvBuf);
DWORD ITEWriteMemAPI(DWORD ulMemAddr, WORD usLength, BYTE* pSrcBuf);
DWORD ITELoadImage(BYTE* pSrcImgBuf, DWORD ulITEImageBufAddr, DWORD ulX, DWORD ulY, DWORD ulW, DWORD ulH);
DWORD ITEDisplayAreaAPI(DWORD ulX, DWORD ulY, DWORD ulW, DWORD ulH, DWORD ulMode, DWORD ulMemAddr, DWORD ulEnWaitReady );
volatile DWORD ITEGetDriveNo(BYTE* pDriveNo);
DWORD ITELdImgAreaAPI(LOAD_IMG_AREA* pstLdImgArea, BYTE* pSrcBuf);
DWORD IT8951PMICCtrlAPI(T_PMIC_CTRL* pstPMICCtrl);
DWORD IT8951SetVComAPI(BYTE ucSetVCom, WORD* pusVComVal);
DWORD IT8951SWPowerSeqAPI(BYTE ucSWPowerOn, WORD usWithSetVComVal);
DWORD IT8951ImgCopyAPI(TDMA_IMG_COPY* pstDMAImgCpy);
DWORD IT8951SFIBlockEraseAPI(TSPICmdArgEraseData* pstArgErase);
DWORD IT8951SFIPageWriteAPI(TSPICmdArgData* pstSPIArg, BYTE* pWBuf);
DWORD IT8951SFIPageReadAPI(TSPICmdArgData* pstSPIArg, BYTE* pRBuf);
DWORD ITEFSetTempCtrlAPI(T_F_SET_TEMP* pstFTempCtrl);
DWORD ITEGetSetTempAPI(BYTE ucSet, BYTE ucSetValue);
DWORD ITEResetTcon();
DWORD ITESendMailBoxCmdArgAPI(TUDefCmdArg* pstUDefCmdArg);
DWORD ITESetTConCfgAPI(BYTE* pCfgFile,  DWORD ulWrMemAddr, DWORD ulSize);
void ITEWaitDpyReady();
DWORD ITECheckDpyReady(DWORD& ulReadyStatus);
DWORD ITEReadMailBoxAPI(BYTE ucMode, BYTE* RBuf, WORD usSize);
DWORD ITEMBGetSystemInfoAPI(HSpiIT8957DevInfo* pstMBSystemInfo);
DWORD ITESetUSBKeepAlive(DWORD dwEnable);
DWORD ITEGetUSBKeepAlive(DWORD& dwEnable);
DWORD ITESetShowOOBEStatus(DWORD dwEnable);
DWORD ITEGetShowOOBEStatus(DWORD& dwEnable);
DWORD ITESetUSBTouchWakeUp(DWORD dwEnable);
DWORD ITEGetUSBTouchWakeUp(DWORD& dwEnable);
//niuxj add
DWORD ITESet8951KeepAlive(DWORD dwEnable);//1-enable, 0-disable

extern HANDLE hDev;

extern BYTE gSPTDataBuf[SPT_BUF_SIZE+1024];

extern DWORD gulPanelW;
extern DWORD gulPanelH;
extern atomic_int gnCmdCnt;
extern atomic_int gnCheckTime;
extern atomic_int gnQuitTransfer;

//LENOVO
DWORD ITESetMIPIModeAPI(DWORD& ulMode);
DWORD ITEGetMIPIModeAPI(DWORD& ulMode);
DWORD ITECleanUpEInkAPI();
DWORD ITEGetBufferAddrInfoAPI(DWORD* dwAddrs);
DWORD ITESetTconDefaultImageAPI(DWORD ulMode, DWORD defaultImg, DWORD ulImageAddr, DWORD recovery, DWORD checkFlag);
DWORD ITEResetTcon();
DWORD ITECpyImageAPI(WORD backup, WORD imageId);
DWORD ITEMipiOfflineTestAPI(WORD offlineType);
DWORD ITEHypPipeLineSetAPI(bool singleEngine);
DWORD ITESetEinkTouchOri(DWORD dwOri);
DWORD ITEGetEinkTouchOri(DWORD& dwOri);
DWORD ITESetTPMaskArea(DWORD dwPenStyle, DWORD dwAreaID, DWORD dwX1, DWORD dwX2, DWORD dwY1, DWORD dwY2);
DWORD ITEGetTPMaskArea(WORD* dwArea);
DWORD ITESetHWArea(DWORD dwAreaID, DWORD dwX1, DWORD dwX2, DWORD dwY1, DWORD dwY2);
DWORD ITESetHWRect1(DWORD dwAreaID, DWORD dwX1, DWORD dwX2, DWORD dwY1, DWORD dwY2);//enable direct handwriting in one rectangle
DWORD ITESetHWRect2(DWORD dwAreaID, DWORD dwX1, DWORD dwX2, DWORD dwX3, DWORD dwX4, DWORD dwY1, DWORD dwY2);//enable direct handwriting in two rectangles which are same in the Y coordinate
DWORD ITEGetHWRect(DWORD& dwCnt, WORD* dwPoint);
DWORD ITESetEinkMinPenWidth(DWORD dwMinW, DWORD dwWidth, DWORD dwOffset, DWORD deOffsetY);//��С�ʿ����ʿ�������ֵ���ʮ�ֽӽ���ѹ�оͻ�û��Ч��
DWORD ITESetEinkPenPressureLevel(DWORD dwPressure);
DWORD ITEEnableHWWriting(DWORD dwPenMode, DWORD bDisableHover=0);
DWORD ITEGetDHWMode(DWORD& dwPenMode); 
DWORD ITESetEinkEraserWidth(DWORD dwMinW, DWORD dwWidth, DWORD dwOffX, DWORD dwOffY);//��С�ʿ����ʿ�������ֵ���ʮ�ֽӽ���ѹ�оͻ�û��Ч��
DWORD ITEGetDHWPenWidth(DWORD& dwMin, DWORD& dwGlobal);
DWORD ITEGetDHWEraserSetting(DWORD& dwMin, DWORD& dwGlobal, DWORD& dwEraser);
DWORD ITESetHighBandWidthRecovery();
DWORD ITESetMIPIModeAPI(DWORD& ulMode);

//WA for load image
void StopLoadImg();
void RecoveryLoadImg();
void EnableLoadImg();
void DisableLoadImg();
DWORD SetPrioritynSendMailBoxCmd(TUDefCmdArg* pstUDefCmdArg);
void SetCheckDpyTimes(DWORD dwVal);

#endif //}__IT8951_USB_API___

