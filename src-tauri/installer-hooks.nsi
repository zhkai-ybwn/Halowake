; Halowake Windows NSIS Installer Hooks
; Comprehensive lifecycle, legacy migration & shortcut management:
; 1. Migrates legacy Lumina installation (cleans up obsolete Lumina directory, registry, shortcuts).
; 2. Enforces single-instance desktop shortcut and cleans up duplicate/stale/cross-context shortcuts.
; 3. Ensures desktop shortcut is always created/updated properly during both clean install and update.

!macro NSIS_HOOK_PREINSTALL
  ; 1. Clean up legacy Lumina desktop and start menu shortcuts
  SetShellVarContext all
  Delete "$DESKTOP\Lumina.lnk"
  Delete "$DESKTOP\Lumina (1).lnk"
  Delete "$DESKTOP\Lumina - 副本.lnk"
  Delete "$DESKTOP\Lumina - 快捷方式.lnk"
  Delete "$SMPROGRAMS\Lumina.lnk"
  RMDir /r "$SMPROGRAMS\Lumina"

  SetShellVarContext current
  Delete "$DESKTOP\Lumina.lnk"
  Delete "$DESKTOP\Lumina (1).lnk"
  Delete "$DESKTOP\Lumina - 副本.lnk"
  Delete "$DESKTOP\Lumina - 快捷方式.lnk"
  Delete "$SMPROGRAMS\Lumina.lnk"
  RMDir /r "$SMPROGRAMS\Lumina"

  ; 2. Clean up duplicate Halowake desktop shortcuts from previous contexts/runs
  SetShellVarContext all
  Delete "$DESKTOP\${PRODUCTNAME}.lnk"
  Delete "$DESKTOP\${PRODUCTNAME} (1).lnk"
  Delete "$DESKTOP\${PRODUCTNAME} - 副本.lnk"
  Delete "$DESKTOP\${PRODUCTNAME} - 快捷方式.lnk"

  SetShellVarContext current
  Delete "$DESKTOP\${PRODUCTNAME} (1).lnk"
  Delete "$DESKTOP\${PRODUCTNAME} - 副本.lnk"
  Delete "$DESKTOP\${PRODUCTNAME} - 快捷方式.lnk"

  ; 3. Clean up legacy Lumina installation directory & registry if it is separate from current $INSTDIR
  ${If} ${FileExists} "$LOCALAPPDATA\Programs\Lumina\Lumina.exe"
    ${If} "$INSTDIR" != "$LOCALAPPDATA\Programs\Lumina"
      RMDir /r "$LOCALAPPDATA\Programs\Lumina"
    ${EndIf}
  ${EndIf}

  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lumina"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\Lumina"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 4. Always ensure current user desktop shortcut points to Halowake.exe
  SetShellVarContext current
  CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
  !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"

  ; 5. Ensure no duplicate on all-users desktop
  SetShellVarContext all
  Delete "$DESKTOP\${PRODUCTNAME}.lnk"
  Delete "$DESKTOP\Lumina.lnk"
  SetShellVarContext current
!macroend
