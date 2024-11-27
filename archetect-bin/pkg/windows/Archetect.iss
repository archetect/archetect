; See https://jrsoftware.org/isinfo.php for more information on Inno Setup
[Setup]
AppName=Archetect
AppVersion={#GetEnv('ARCHETECT_VERSION')}
AppPublisher=Archetect
AppPublisherURL=https://github.com/archetect
DefaultDirName={autopf}\Archetect
DefaultGroupName=Archetect
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
AllowNoIcons=yes
OutputBaseFilename=archetect-installer
Compression=zip
SolidCompression=no
WizardStyle=modern
SourceDir={#GetEnv('GITHUB_WORKSPACE')}
OutputDir=.

[Files]
Source: "{#GetEnv('ARCHETECT_BIN')}"; DestDir: "{app}"; Flags: ignoreversion

[Run]
Filename: "{cmd}"; Parameters: "/c archetect --version || setx PATH ""%PATH%;{app};"""; Flags: runhidden; StatusMsg: "Adding Archetect to PATH..."