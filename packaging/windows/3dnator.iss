; Instalador Windows: bibliotecas CAD ficam adjacentes e substituíveis.
; AppVersion e PackageDir são recebidos do workflow, nunca do ambiente instalado.
#ifndef AppVersion
  #error AppVersion precisa ser informado.
#endif
#ifndef PackageDir
  #error PackageDir precisa ser informado.
#endif
#ifndef RedistPath
  #error RedistPath precisa ser informado.
#endif
[Setup]
AppId={{FF933799-6389-46D6-9289-3752B684AFAB}
AppName=3Dnator
AppVersion={#AppVersion}
VersionInfoVersion={#AppNumericVersion}
AppPublisher=3Dnator
AppPublisherURL=https://github.com/trocolis/3dnator
DefaultDirName={autopf}\3Dnator
DefaultGroupName=3Dnator
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputBaseFilename=3dnator-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\3dnator.exe
LicenseFile=..\..\LICENSE
[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"
[Files]
Source: "{#PackageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RedistPath}"; DestDir: "{tmp}"; Flags: deleteafterinstall
[Icons]
Name: "{group}\3Dnator"; Filename: "{app}\3dnator.exe"
[Run]
; Redistribuível oficial Microsoft, com seus próprios termos e assinatura.
Filename: "{tmp}\vc_redist.x64.exe"; Parameters: "/install /quiet /norestart"; Flags: waituntilterminated
Filename: "{app}\3dnator.exe"; Description: "Iniciar 3Dnator"; Flags: nowait postinstall skipifsilent
