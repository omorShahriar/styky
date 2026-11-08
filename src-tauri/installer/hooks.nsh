; Installer hooks (bundle.windows.nsis.installerHooks): put the `styky` CLI on the user PATH.
; The CLI ships as $INSTDIR\bin\styky.exe together with styky-path.ps1 (both are bundle resources).

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\bin\styky-path.ps1" -Action add -Dir "$INSTDIR\bin"'
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::ExecToLog 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$INSTDIR\bin\styky-path.ps1" -Action remove -Dir "$INSTDIR\bin"'
  Pop $0
!macroend
