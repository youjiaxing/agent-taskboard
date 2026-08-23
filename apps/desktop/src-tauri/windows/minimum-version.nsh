!include "WinVer.nsh"

!macro NSIS_HOOK_PREINSTALL
  ${IfNot} ${AtLeastBuild} 17763
    MessageBox MB_ICONSTOP|MB_OK "Agent Taskboard requires Windows 10 version 1809 or later."
    Quit
  ${EndIf}
!macroend
