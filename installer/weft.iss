; Inno Setup 6 script for WEFT Desktop
; CI overrides AppVersion via /DAppVersion="x.y.z"

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppName=WEFT
AppVersion={#AppVersion}
AppPublisher=WEFT
AppPublisherURL=https://github.com/ailiheizi/weft
DefaultDirName={autopf}\WEFT
DefaultGroupName=WEFT
UninstallDisplayIcon={app}\weft_client.exe
OutputDir=..\installer-output
OutputBaseFilename=weft-setup-{#AppVersion}
Compression=lzma2/ultra
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
PrivilegesRequired=admin
SetupIconFile=compiler:SetupClassicIcon.ico

[Files]
Source: "..\weft-desktop-windows-x86_64\*"; DestDir: "{app}"; Flags: recursesubdirs createallsubdirs

[Icons]
Name: "{group}\WEFT"; Filename: "{app}\weft_client.exe"
Name: "{group}\Uninstall WEFT"; Filename: "{uninstallexe}"
Name: "{commondesktop}\WEFT"; Filename: "{app}\weft_client.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Run]
Filename: "{app}\weft_client.exe"; Description: "Launch WEFT"; Flags: nowait postinstall skipifsilent
