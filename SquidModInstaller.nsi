Name "SquidMod"
!ifndef OUTFILE
  !define OUTFILE "SquidModInstaller.exe"
!endif
OutFile "${OUTFILE}"
InstallDir "$PROGRAMFILES64\SquidMod"
RequestExecutionLevel admin   ; Needed to write to Program Files

Page directory
Page instfiles

!define PKG_DIR "pkg"
!define APP_VERSION "${VERSION}"

Section "Install Files"

    SetOutPath "$INSTDIR"

    File /r "${PKG_DIR}\*.*"

    ; Create shortcuts
    CreateDirectory "$SMPROGRAMS\SquidMod"
    CreateShortCut "$DESKTOP\SquidMod.lnk" "$INSTDIR\SquidMod.exe" "" "$INSTDIR\icon.ico"
    CreateShortCut "$SMPROGRAMS\SquidMod\SquidMod.lnk" "$INSTDIR\SquidMod.exe" "" "$INSTDIR\icon.ico"

    WriteUninstaller "$INSTDIR\Uninstall.exe"

    ; Register in Add/Remove Programs
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SquidMod" "DisplayName" "SquidMod"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SquidMod" "UninstallString" "$INSTDIR\Uninstall.exe"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SquidMod" "DisplayIcon" "$INSTDIR\icon.ico"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SquidMod" "DisplayVersion" "${APP_VERSION}"
    WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SquidMod" "Publisher" "Jerry Starke"

SectionEnd

Section "Uninstall"

    Delete "$INSTDIR\SquidMod.exe"
    Delete "$INSTDIR\icon.ico"

    RMDir /r "$INSTDIR"

    Delete "$DESKTOP\SquidMod.lnk"
    Delete "$SMPROGRAMS\SquidMod\SquidMod.lnk"
    RMDir "$SMPROGRAMS\SquidMod"

    ; Remove registry keys on uninstall
    DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\SquidMod"

SectionEnd
