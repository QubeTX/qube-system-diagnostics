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

[Registry]
; Install-source marker. sd300 update reads HKCU\Software\SD300\InstallSource
; and picks the matching installer to download for in-place upgrades. Value
; must match the `exe-corporate` arm in src/update.rs.
Root: HKCU; Subkey: "Software\SD300"; ValueType: string; ValueName: "InstallSource"; ValueData: "exe-corporate"; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\SD300"; ValueType: string; ValueName: "InstallSourceCorporate"; ValueData: "exe-corporate"; Flags: uninsdeletevalue
Root: HKCU; Subkey: "Software\SD300"; Flags: uninsdeletekeyifempty

[Code]
#define ConflictingMsiDisplayName MyAppFullName
#define ConflictingMsiPublisher MyAppPublisher
#define OtherEditionDisplayName "SD-300 Global"
#define OtherEditionInnoAppId "{DC74D35F-CBF4-425F-B11E-E9EA87C13CA9}"
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
    EnvAddPath(ExpandConstant('{app}') + '\bin');
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    EnvRemovePath(ExpandConstant('{app}') + '\bin');
end;
