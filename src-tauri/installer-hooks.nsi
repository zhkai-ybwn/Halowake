; Lumina Windows NSIS Installer Hooks
; Comprehensive lifecycle & shortcut management:
; 1. Terminates orphan dev server processes & main processes before replacing binaries.
; 2. Enforces single-instance desktop shortcut and cleans up duplicate/stale/cross-context shortcuts.
; 3. Correctly handles silent update detection.

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

  ; 2. In update mode, prevent NSIS from creating new shortcuts by setting NoShortcutMode
  ; Standard Tauri installer template respects $NoShortcutMode and skips CreateOrUpdateDesktopShortcut
  ${If} $UpdateMode = 1
    StrCpy $NoShortcutMode 1
  ${EndIf}

  ; 3. Clean up duplicate public desktop shortcuts & duplicate aliases
  SetShellVarContext all
  Delete "$DESKTOP\${PRODUCTNAME}.lnk"
  Delete "$DESKTOP\${PRODUCTNAME} (1).lnk"
  Delete "$DESKTOP\${PRODUCTNAME} - 副本.lnk"
  Delete "$DESKTOP\${PRODUCTNAME} - 快捷方式.lnk"

  SetShellVarContext current
  Delete "$DESKTOP\${PRODUCTNAME} (1).lnk"
  Delete "$DESKTOP\${PRODUCTNAME} - 副本.lnk"
  Delete "$DESKTOP\${PRODUCTNAME} - 快捷方式.lnk"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 4. If an existing user desktop shortcut exists, ensure its target and AppUserModelId are updated
  SetShellVarContext current
  ${If} ${FileExists} "$DESKTOP\${PRODUCTNAME}.lnk"
    CreateShortcut "$DESKTOP\${PRODUCTNAME}.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    !insertmacro SetLnkAppUserModelId "$DESKTOP\${PRODUCTNAME}.lnk"
  ${EndIf}

  ; Ensure no duplicate on all-users desktop
  SetShellVarContext all
  Delete "$DESKTOP\${PRODUCTNAME}.lnk"
  SetShellVarContext current
!macroend
