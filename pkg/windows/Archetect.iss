; See https://jrsoftware.org/isinfo.php for more information on Inno Setup
#if Ver < EncodeVer(6,0,0,0)
  #error This script requires Inno Setup 6 or later
#endif

[Setup]
AlwaysRestart=true
AppId={{65a4900f-d54c-45a4-8645-82db7cb5266b}
AppName=Archetect
AppVersion={#GetEnv('VERSION')}
AppPublisher=Archetect
AppPublisherURL=https://github.com/archetect
ChangesEnvironment=yes
DefaultDirName={autopf}\Archetect
DefaultGroupName=Archetect
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
AllowNoIcons=yes
Compression=zip
SolidCompression=no
WizardStyle=modern
SourceDir={#GetEnv('GITHUB_WORKSPACE')}
OutputBaseFilename=archetect-{#GetEnv('VERSION')}-windows_x64-installer
OutputDir=.\target
UsePreviousAppDir=false
UninstallDisplayName=Archetect
Uninstallable=true

[Files]
Source: "pkg\windows\PathMgr.dll"; DestDir: "{app}"; Flags: uninsneveruninstall
Source: "target\release\archetect.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "LICENSE"; DestDir: "{app}"; Flags: ignoreversion

; Enable Long Paths Support in the Registry
[Registry]
Root: HKLM; Subkey: "SYSTEM\CurrentControlSet\Control\FileSystem"; ValueType: dword; ValueName: "LongPathsEnabled"; ValueData: "1"

[Run]
Filename: "git.exe"; Parameters: "config --global core.longpaths true"; Description: "Set up Git"; Flags: runhidden

[Tasks]
Name: modifypath; Description: "&Add to Path"

[Code]
const
  MODIFY_PATH_TASK_NAME = 'modifypath';  // Specify name of task

var
  PathIsModified: Boolean;          // Cache task selection from previous installs
  ApplicationUninstalled: Boolean;  // Has application been uninstalled?

// Import AddDirToPath() at setup time ('files:' prefix)
function DLLAddDirToPath(DirName: string; PathType, AddType: DWORD): DWORD;
  external 'AddDirToPath@files:PathMgr.dll stdcall delayload setuponly';

// Import RemoveDirFromPath() at uninstall time ('{app}\' prefix)
function DLLRemoveDirFromPath(DirName: string; PathType: DWORD): DWORD;
  external 'RemoveDirFromPath@{app}\PathMgr.dll stdcall uninstallonly';

// Wrapper for AddDirToPath() DLL function
function AddDirToPath(const DirName: string): DWORD;
var
  PathType, AddType: DWORD;
begin
  // PathType = 0 - use system Path
  // PathType = 1 - use user Path
  // AddType = 0 - add to end of Path
  // AddType = 1 - add to beginning of Path
  if IsAdminInstallMode() then
    PathType := 0
  else
    PathType := 1;
  AddType := 0;
  result := DLLAddDirToPath(DirName, PathType, AddType);
end;

// Wrapper for RemoveDirFromPath() DLL function
function RemoveDirFromPath(const DirName: string): DWORD;
var
  PathType: DWORD;
begin
  // PathType = 0 - use system Path
  // PathType = 1 - use user Path
  if IsAdminInstallMode() then
    PathType := 0
  else
    PathType := 1;
  result := DLLRemoveDirFromPath(DirName, PathType);
end;

procedure RegisterPreviousData(PreviousDataKey: Integer);
begin
  // Store previous or current task selection as custom user setting
  if PathIsModified or WizardIsTaskSelected(MODIFY_PATH_TASK_NAME) then
    SetPreviousData(PreviousDataKey, MODIFY_PATH_TASK_NAME, 'true');
end;

function InitializeSetup(): Boolean;
begin
  result := true;
  // Was task selected during a previous install?
  PathIsModified := GetPreviousData(MODIFY_PATH_TASK_NAME, '') = 'true';
end;

function InitializeUninstall(): Boolean;
begin
  result := true;
  // Was task selected during a previous install?
  PathIsModified := GetPreviousData(MODIFY_PATH_TASK_NAME, '') = 'true';
  ApplicationUninstalled := false;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    // Add app directory to Path at post-install step if task selected
    if PathIsModified or WizardIsTaskSelected(MODIFY_PATH_TASK_NAME) then
      AddDirToPath(ExpandConstant('{app}'));
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    // Remove app directory from path during uninstall if task was selected;
    // use variable because we can't use WizardIsTaskSelected() at uninstall
    if PathIsModified then
      RemoveDirFromPath(ExpandConstant('{app}'));
  end
  else if CurUninstallStep = usPostUninstall then
  begin
    ApplicationUninstalled := true;
  end;
end;

procedure DeinitializeUninstall();
begin
  if ApplicationUninstalled then
  begin
    // Unload and delete PathMgr.dll and remove app dir when uninstalling
    UnloadDLL(ExpandConstant('{app}\PathMgr.dll'));
    DeleteFile(ExpandConstant('{app}\PathMgr.dll'));
    RemoveDir(ExpandConstant('{app}'));
  end;
end;