; Replace the known old per-user program tree; preserve all user data.
; Current-user data checkbox and autostart cleanup.
; Installation, updates, shortcuts and removal belong to Tauri's NSIS template.
!include LogicLib.nsh

; Refuse junctions/symlinks before deleting any part of a known program tree.
; The current data directories are outside these trees and are never migrated.
!macro GovorunPlainTree PREFIX
Function ${PREFIX}GovorunPlainTree
  Exch $0
  Push $1
  Push $2
  Push $3
  StrLen $1 $0
  ${If} $1 > 900
    SetErrors
    Goto done
  ${EndIf}
  System::Call 'kernel32::GetFileAttributesW(w r0) i.r1 ?e'
  Pop $3
  ${If} $1 = -1
  ${AndIf} $3 != 2
  ${AndIf} $3 != 3
    SetErrors
    Goto done
  ${EndIf}
  ${If} $1 != -1
    IntOp $2 $1 & 0x400
    ${If} $2 != 0
      SetErrors
      Goto done
    ${EndIf}
    IntOp $2 $1 & 0x10
    ${If} $2 != 0
      FindFirst $1 $2 "$0\*"
      ${If} ${Errors}
        Goto done
      ${EndIf}
      ${DoWhile} $2 != ""
        ${If} $2 != "."
        ${AndIf} $2 != ".."
          Push "$0\$2"
          Call ${PREFIX}GovorunPlainTree
          ${If} ${Errors}
            FindClose $1
            SetErrors
            Goto done
          ${EndIf}
        ${EndIf}
        FindNext $1 $2
      ${Loop}
      FindClose $1
    ${EndIf}
  ${EndIf}
  ClearErrors
done:
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd
!macroend
!insertmacro GovorunPlainTree ""
!insertmacro GovorunPlainTree "un."

!macro GovorunProfile
  !if "${BUNDLEID}" == "com.govorun.app"
    StrCpy $R6 "Govorun"
    StrCpy $R7 "Говорун"
  !else if "${BUNDLEID}" == "com.govorun.app.testbackend"
    StrCpy $R6 "Govorun.TestBackend"
    StrCpy $R7 "Говорун Тест"
  !else
    !error "Unsupported Govorun identity"
  !endif
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro GovorunProfile
  ${If} $INSTDIR == "$LOCALAPPDATA\Programs\$R6"
    MessageBox MB_OK|MB_ICONEXCLAMATION "Выберите новую папку установки по умолчанию. Прежние файлы будут удалены после установки, данные сохранятся." /SD IDOK
    SetErrorLevel 2
    Abort
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro GovorunProfile
  StrCpy $R8 "$LOCALAPPDATA\Programs\$R6"
  ReadRegStr $R9 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\$R6" "InstallLocation"
  ; Never execute the broken former app or its uninstaller. A registry entry
  ; for another location is not authority to delete that location.
  StrCpy $R4 0
  ${If} $R9 == $R8
    StrCpy $R4 1
  ${ElseIf} $R9 == ""
    ${If} ${FileExists} "$R8\Govorun.exe"
      StrCpy $R4 1
    ${EndIf}
  ${EndIf}
  ${If} $R4 = 1
    ClearErrors
    System::Call 'kernel32::GetFileAttributesW(w "$LOCALAPPDATA") i.r0'
    IntOp $0 $0 & 0x400
    ${If} $0 != 0
      SetErrors
    ${Else}
      System::Call 'kernel32::GetFileAttributesW(w "$LOCALAPPDATA\Programs") i.r0'
      IntOp $0 $0 & 0x400
      ${If} $0 != 0
        SetErrors
      ${Else}
        Push $R8
        Call GovorunPlainTree
      ${EndIf}
    ${EndIf}
    ${If} ${Errors}
      MessageBox MB_OK|MB_ICONEXCLAMATION "Старая папка содержит ссылку на другой каталог. Новая версия установлена; старая папка сохранена." /SD IDOK
    ${Else}
      ClearErrors
      ${If} ${FileExists} "$R8\*.*"
        RMDir /r "$R8"
      ${EndIf}
      ${If} ${Errors}
        MessageBox MB_OK|MB_ICONEXCLAMATION "Новая версия установлена. Закройте прежнего Говоруна и повторите установку, чтобы удалить старые файлы." /SD IDOK
      ${Else}
        ${If} $R9 == $R8
          DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\$R6"
        ${EndIf}
        !insertmacro IsShortcutTarget "$DESKTOP\$R7.lnk" "$R8\Govorun.exe"
        Pop $0
        ${If} $0 = 1
          Delete "$DESKTOP\$R7.lnk"
        ${EndIf}
        !insertmacro IsShortcutTarget "$SMPROGRAMS\$R7\$R7.lnk" "$R8\Govorun.exe"
        Pop $0
        ${If} $0 = 1
          Delete "$SMPROGRAMS\$R7\$R7.lnk"
          RMDir "$SMPROGRAMS\$R7"
        ${EndIf}
        ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "$R6"
        ${If} $0 == '$\"$R8\Govorun.exe$\" --background'
        ${OrIf} $0 == '$\"$R8\Govorun.exe$\"'
          DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "$R6"
          WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "$R6" '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --background'
        ${EndIf}
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; A model-free update can leave the old bundled model outside the new list
  ; of Tauri resources. Remove only this app-owned directory on real uninstall.
  ; During an update the retained model must remain available.
  ${If} $UpdateMode <> 1
    Push "$INSTDIR"
    Call un.GovorunPlainTree
    ${IfNot} ${Errors}
      RMDir /r "$INSTDIR\native\Models"
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro GovorunProfile
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "$R6"
  ${If} $UpdateMode <> 1
  ${AndIf} $0 == '$\"$INSTDIR\${MAINBINARYNAME}.exe$\" --background'
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "$R6"
  ${EndIf}
  ; Tauri already handles its own data checkbox and never selects it on update.
  ; The same explicit choice covers native data and the retained model cache.
  ${If} $DeleteAppDataCheckboxState = 1
  ${AndIf} $UpdateMode <> 1
    System::Call 'kernel32::GetFileAttributesW(w "$LOCALAPPDATA") i.r0'
    IntOp $0 $0 & 0x400
    ${If} $0 != 0
      Abort
    ${EndIf}
    Push "$LOCALAPPDATA\$R6"
    Call un.GovorunPlainTree
    ${IfNot} ${Errors}
      RMDir /r "$LOCALAPPDATA\$R6"
    ${EndIf}
    Push "$LOCALAPPDATA\${BUNDLEID}.models"
    Call un.GovorunPlainTree
    ${IfNot} ${Errors}
      RMDir /r "$LOCALAPPDATA\${BUNDLEID}.models"
    ${EndIf}
  ${EndIf}
!macroend
