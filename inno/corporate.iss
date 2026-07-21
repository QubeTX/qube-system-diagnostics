; SD-300 — Corporate Edition installer (perUser, no admin required).
;
; Built by .github/workflows/windows-installers.yml after release.yml finishes
; publishing the GitHub Release. Companion to inno/global.iss (perMachine
; sibling). The MSI sibling lives at wix/corporate.wxs.
;
; Builds with Inno Setup 6 (`iscc` from JRSoftware) on a Windows runner. CI
; passes the version via `iscc /DMyAppVersion=3.15.0`.
;
; All four installers (MSI Global, MSI Corporate, EXE Global, EXE Corporate)
; target only TWO actual install paths — one per edition. The Corporate
; edition (both MSI and EXE) installs to
;     %LocalAppData%\Programs\sd300\bin\sd300.exe
; and modifies USER PATH (HKCU\Environment\Path), not system PATH. A fresh
; explicit Inno launch removes the same-edition MSI first, so the newest format
; choice owns one binary and one Add/Remove Programs registration.

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0-dev"
#endif
#ifndef MyAppBinaryDir
  #define MyAppBinaryDir "..\target\release"
#endif
#ifndef MyAppGuiBinDir
  #define MyAppGuiBinDir "..\target\native-gui-stage\windows-x86_64\app\zig-out\bin"
#endif

#define MyAppName "sd300"
#define MyAppFullName "SD-300 Corporate"
#define MyAppPublisher "Emmett S"
#define MyAppURL "https://github.com/QubeTX/qube-system-diagnostics"
#define MyAppExeName "sd300.exe"

[Setup]
; AppId is the immutable identity of the Corporate EXE installer.
; Different GUID from both the Corporate MSI's UpgradeCode and the Global
; EXE's AppId so the four installer products are distinct to Windows.
AppId={{ED209931-B5C0-43AE-89F6-83EE2C581653}
AppName={#MyAppFullName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppFullName} {#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}/releases
; perUser install location: %LocalAppData%\Programs\sd300 — same path as the
; Corporate MSI by design. {userpf} is Inno Setup's per-user "Program Files"
; equivalent and resolves to %LocalAppData%\Programs.
DefaultDirName={userpf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
; The core perUser switch: lowest privileges, no admin elevation, no UAC.
; PrivilegesRequiredOverridesAllowed= prevents Inno from offering the user
; the choice to elevate (we deliberately install per-user only).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
OutputBaseFilename=sd300-windows-x64-corporate
OutputDir=Output
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes
; ARP display name. Matches the Corporate MSI's Product Name so the two
; installer formats show consistent labels.
UninstallDisplayName={#MyAppFullName}
LicenseFile=..\LICENSE.md
SetupLogging=yes
; Cross-method consolidation (v3.17.0+) - see inno/global.iss for rationale.
AppMutex=SD300_Running
CloseApplications=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "{#MyAppBinaryDir}\{#MyAppExeName}"; DestDir: "{app}\bin"; Flags: ignoreversion; AfterInstall: ConsolidatePriorCli
Source: "{#MyAppGuiBinDir}\sd300-gui.exe"; DestDir: "{app}\app"; Flags: ignoreversion
Source: "{#MyAppGuiBinDir}\sd300_engine.dll"; DestDir: "{app}\app"; Flags: ignoreversion
Source: "{#MyAppGuiBinDir}\assets\icon.png"; DestDir: "{app}\app\assets"; Flags: ignoreversion
Source: "{#MyAppGuiBinDir}\licenses\*"; DestDir: "{app}\app\licenses"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\SD-300"; Filename: "{app}\app\sd300-gui.exe"; WorkingDir: "{app}\app"; Comment: "Open the SD-300 native system monitor"

[Registry]
; Install-source marker. sd300 update reads HKCU\Software\SD300\InstallSource
; and picks the matching installer to download for in-place upgrades. Value
; must match the `exe-corporate` arm in src/update.rs.
Root: HKCU; Subkey: "Software\SD300"; ValueType: string; ValueName: "InstallSource"; ValueData: "exe-corporate"; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\SD300"; ValueType: string; ValueName: "InstallSourceCorporate"; ValueData: "exe-corporate"; Flags: uninsdeletevalue
; Per-user ownership evidence used only to locate the existing v3 CLI for a
; hidden, authenticated GUI shutdown before Setup mutates the shared image.
Root: HKCU; Subkey: "Software\SD300"; ValueType: string; ValueName: "NativeInstallRootCorporate"; ValueData: "{app}\"; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\SD300"; Flags: uninsdeletekeyifempty

[Code]
#define ConflictingMsiDisplayName MyAppFullName
#define ConflictingMsiPublisher MyAppPublisher
#define OtherEditionDisplayName "SD-300 Global"
#define OtherEditionInnoAppId "{DC74D35F-CBF4-425F-B11E-E9EA87C13CA9}"

procedure StopGuiAtRoot(Root: String);
var
  ExitCode: Integer;
  CliBinary: String;
  GuiBinary: String;
begin
  Root := RemoveBackslashUnlessRoot(Root);
  GuiBinary := Root + '\app\sd300-gui.exe';
  if not FileExists(GuiBinary) then
    exit;
  CliBinary := Root + '\bin\{#MyAppExeName}';
  if not FileExists(CliBinary) then
    RaiseException('An owned SD-300 GUI exists without its lifecycle CLI. Setup stopped before changing files.');
  if not Exec(CliBinary, 'stop-gui --quiet', ExtractFileDir(CliBinary),
      SW_HIDE, ewWaitUntilTerminated, ExitCode) then
    RaiseException('Could not start the hidden SD-300 GUI lifecycle request. Setup stopped safely.');
  if ExitCode <> 0 then
    RaiseException('The SD-300 GUI did not stop safely (exit ' + IntToStr(ExitCode) + ').');
end;

procedure StopExistingGui;
var
  RegisteredRoot: String;
begin
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Software\SD300',
      'NativeInstallRootCorporate', RegisteredRoot) then
    StopGuiAtRoot(RegisteredRoot);
end;

#include "remove-conflicting-msi.pas"

procedure RunStrictMigration(Args: String; LabelText: String);
var
  ExitCode: Integer;
  Binary: String;
begin
  Binary := ExpandConstant('{app}\bin\{#MyAppExeName}');
  if not Exec(Binary, 'migrate-cleanup --quiet --strict ' + Args,
      ExpandConstant('{app}\bin'), SW_HIDE, ewWaitUntilTerminated, ExitCode) then
    RaiseException('Could not start SD-300 ' + LabelText + ' cleanup. Setup stopped safely.');
  if ExitCode <> 0 then
    RaiseException('SD-300 ' + LabelText + ' cleanup did not converge (exit ' +
      IntToStr(ExitCode) + '). Setup cannot claim one active install. Use ' +
      'https://github.com/QubeTX/qube-system-diagnostics/releases/latest');
end;

procedure ConsolidatePriorCli;
begin
  { A registered Global product was rejected by PrepareToInstall. Remove any
    orphan first, then the current user's Cargo/managed copy. }
  RunStrictMigration('--other-edition', 'orphaned-edition');
  RunStrictMigration('--cargo-copy', 'managed/Cargo');
end;

procedure VerifyGuiCompanion;
var
  ExitCode: Integer;
  GuiBinary: String;
begin
  GuiBinary := ExpandConstant('{app}\app\sd300-gui.exe');
  if not Exec(GuiBinary, '--self-test --json', ExpandConstant('{app}\app'),
      SW_HIDE, ewWaitUntilTerminated, ExitCode) then
    RaiseException('Could not start the installed SD-300 GUI self-test. Setup stopped safely.');
  if ExitCode <> 0 then
    RaiseException('The installed SD-300 GUI or engine failed self-test (exit ' +
      IntToStr(ExitCode) + '). Setup stopped safely.');
end;

function HasCommandLineParameter(Parameter: String): Boolean;
var
  Index: Integer;
begin
  Result := False;
  for Index := 1 to ParamCount do
    if CompareText(ParamStr(Index), Parameter) = 0 then
    begin
      Result := True;
      exit;
    end;
end;

procedure CleanupOwnedGuiState;
var
  ExitCode: Integer;
  CliBinary: String;
begin
  if HasCommandLineParameter('/PRESERVEGUISTATE') then
  begin
    Log('Preserving GUI state during an installer-format transition.');
    exit;
  end;
  CliBinary := ExpandConstant('{app}\bin\{#MyAppExeName}');
  if not Exec(CliBinary, 'cleanup-gui-state --quiet', ExpandConstant('{app}\bin'),
      SW_HIDE, ewWaitUntilTerminated, ExitCode) then
    RaiseException('Could not start SD-300 GUI state cleanup. Uninstall stopped safely.');
  if ExitCode <> 0 then
    RaiseException('SD-300 GUI state cleanup failed safely (exit ' + IntToStr(ExitCode) + ').');
end;

{
  PATH management — user PATH (HKCU\Environment\Path) for the Corporate
  perUser edition. Same canonical pattern as inno/global.iss but pointing
  at the HKCU\Environment key instead of HKLM\...\Session Manager\Environment.
}
const
  EnvironmentKey = 'Environment';

procedure EnvAddPath(Path: string);
var
  Paths: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    Paths := '';

  // Skip if already in PATH (case-insensitive substring match with
  // ;-padding so we don't match a prefix of a different directory).
  if Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';') > 0 then exit;

  if Length(Paths) > 0 then
    Paths := Paths + ';' + Path
  else
    Paths := Path;

  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths);
end;

procedure EnvRemovePath(Path: string);
var
  Paths: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths) then
    exit;

  P := Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';');
  if P = 0 then exit;

  if P = 1 then
    // First-entry case (audit finding F9, v3.15.6+). Pre-v3.15.6 the
    // line below used Delete(Paths, P - 1, ...) = Delete(Paths, 0, ...)
    // which is undefined behavior in Pascal Script (treated as no-op
    // in Inno Setup's runtime), stranding the entry in PATH after
    // uninstall. Most likely on fresh corporate workstations where
    // HKCU\Environment\Path is empty before install, so SD-300's bin
    // lands at index 1. With this branch:
    //   Paths = "X;Y"  → "Y"    (eats "X;")
    //   Paths = "X;"   → ""     (eats "X;")
    //   Paths = "X"    → ""     (eats "X", count clamps to remaining)
    Delete(Paths, 1, Length(Path) + 1)
  else
    // Middle/end entry: consume the leading `;` plus the path.
    //   Paths = "A;X;B" → "A;B" (eats ";X")
    //   Paths = "A;X"   → "A"   (eats ";X")
    Delete(Paths, P - 1, Length(Path) + 1);

  RegWriteExpandStringValue(HKEY_CURRENT_USER, EnvironmentKey, 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    EnvAddPath(ExpandConstant('{app}') + '\bin');
    VerifyGuiCompanion;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    { The uninstaller itself proves ownership of its application directory;
      it does not need the registry locator required by a fresh installer. }
    StopGuiAtRoot(ExpandConstant('{app}'));
    CleanupOwnedGuiState;
  end;
  if CurUninstallStep = usPostUninstall then
    EnvRemovePath(ExpandConstant('{app}') + '\bin');
end;
