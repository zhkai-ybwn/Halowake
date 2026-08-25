; Lumina Windows NSIS Installer Hooks
; Handles updater edge cases: prevents duplicate desktop shortcuts and cleans up stale/cross-context shortcuts.

!macro NSIS_HOOK_PREINSTALL
  ; 1. Detect if this is an update/reinstall (even if /UPDATE was omitted by silent updater)
  ${If} ${FileExists} "$INSTDIR\${MAINBINARYNAME}.exe"
    StrCpy $UpdateMode 1
  ${Else}
    ReadRegStr $0 SHCTX "${UNINSTKEY}" "UninstallString"
    ${If} $0 != ""
      StrCpy $UpdateMode 1
    ${EndIf}
  ${EndIf}

  ; 2. Clean up duplicate public desktop shortcut if installing in currentUser context
  !if "${INSTALLMODE}" == "currentUser"
    SetShellVarContext all
    ${If} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk"
      Delete "$DESKTOP\${PRODUCTNAME}.lnk"
    ${EndIf}
    SetShellVarContext current
  !endif
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 3. If an existing desktop shortcut is present, ensure its target and AppUserModelId are refreshed
  ${If} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk"
    CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
  ${EndIf}
!macroend
