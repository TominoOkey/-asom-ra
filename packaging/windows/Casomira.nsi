Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"

!ifndef AppVersion
  !define AppVersion "0.1.0"
!endif

Var LogPath
Var PowerShellPath

Name "Časomíra"
Caption "Instalace Časomíry"
OutFile "../../dist/windows/Casomira-Setup-${AppVersion}.exe"
InstallDir "$PROGRAMFILES64\Casomira"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
ShowInstDetails nevershow
ShowUnInstDetails nevershow

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Czech"

Section "Install"
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File "install-com0com.ps1"
  ExpandEnvStrings $LogPath "%TEMP%\Casomira-com0com-install.log"
  FileOpen $1 "$LogPath" w
  FileWrite $1 "Instalacni skript se spousti.$\r$\n"
  FileClose $1
  ${If} ${RunningX64}
    StrCpy $PowerShellPath "$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
  ${Else}
    StrCpy $PowerShellPath "$SYSDIR\WindowsPowerShell\v1.0\powershell.exe"
  ${EndIf}
  IfFileExists "$PowerShellPath" +3 0
    MessageBox MB_ICONSTOP|MB_OK "Windows PowerShell nebyl nalezen: $PowerShellPath"
    Abort
  nsExec::ExecToStack '"$PowerShellPath" -NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "$PLUGINSDIR\install-com0com.ps1" -LogPath "$LogPath"'
  Pop $0
  Pop $1
  ${If} $1 != ""
    FileOpen $2 "$LogPath" a
    FileWrite $2 "$\r$\nPowerShell výstup:$\r$\n$1$\r$\n"
    FileClose $2
  ${EndIf}
  ${If} $0 != 0
    MessageBox MB_ICONSTOP|MB_OK "Instalace ovladače nebo vytvoření páru COM19 ↔ COM20 selhalo (kód: $0). Instalace aplikace byla zrušena. Podrobnosti: $LogPath"
    Abort
  ${EndIf}

  SetOutPath "$INSTDIR"
  File "../../target/x86_64-pc-windows-msvc/release/casomira.exe"
  CreateDirectory "$SMPROGRAMS\Časomíra"
  CreateShortcut "$SMPROGRAMS\Časomíra\Časomíra.lnk" "$INSTDIR\casomira.exe"
  CreateShortcut "$DESKTOP\Časomíra.lnk" "$INSTDIR\casomira.exe"

  WriteUninstaller "$INSTDIR\Uninstall.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Casomira" "DisplayName" "Časomíra"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Casomira" "UninstallString" '"$INSTDIR\Uninstall.exe"'
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\Časomíra\Časomíra.lnk"
  RMDir "$SMPROGRAMS\Časomíra"
  Delete "$DESKTOP\Časomíra.lnk"
  Delete "$INSTDIR\casomira.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Casomira"
SectionEnd
