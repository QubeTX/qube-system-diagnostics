{ Remove same-edition MSI registrations before an explicit Inno install. This
  runs from PrepareToInstall, before Inno writes any files, so the fresh
  installer is the user's latest channel choice without leaving a second
  Windows Installer registration behind.

  Do not call MsiEnumRelatedProductsW directly from Pascal Script here. Hosted
  Inno Setup 6.7.1 proved that the output-buffer ABI can access-violate Setup
  even when the declaration looks correct. Instead, use Inno's supported
  registry helpers and require exact native Add/Remove Programs evidence:
  display name, publisher, WindowsInstaller=1, and a GUID product key.
  The calling .iss defines ConflictingMsiDisplayName,
  ConflictingMsiPublisher, OtherEditionDisplayName, and
  OtherEditionInnoAppId. A different edition/scope is not silently reduced to
  an orphaned registration: the installer stops before mutation and points to
  the managed CLI/release recovery path.

  Do not infer MSI install scope from the Add/Remove Programs hive. A hosted
  runner and the Alienware both registered the no-UAC Corporate per-user MSI
  under HKLM64 while its payload and marker remained user-scoped. Search both
  hives/views, then let the exact edition identity and product code govern. }

const
  ErrorSuccess = 0;
  ErrorUnknownProduct = 1605;
  ErrorSuccessRebootInitiated = 1641;
  ErrorSuccessRebootRequired = 3010;
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall';
  MaxConflictingMsiProducts = 32;

function IsHexDigit(Value: Char): Boolean;
begin
  Result := ((Value >= '0') and (Value <= '9')) or
    ((Value >= 'A') and (Value <= 'F')) or
    ((Value >= 'a') and (Value <= 'f'));
end;

function IsProductCode(Value: String): Boolean;
var
  Index: Integer;
begin
  Result := False;
  if (Length(Value) <> 38) or (Value[1] <> '{') or (Value[38] <> '}') then
    exit;

  for Index := 2 to 37 do
  begin
    if (Index = 10) or (Index = 15) or (Index = 20) or (Index = 25) then
    begin
      if Value[Index] <> '-' then
        exit;
    end
    else if not IsHexDigit(Value[Index]) then
      exit;
  end;
  Result := True;
end;

function HasMatchingMsiProduct(RootKey: Integer; DisplayNameToMatch: String): Boolean;
var
  Subkeys: TArrayOfString;
  Key: String;
  DisplayName: String;
  Publisher: String;
  WindowsInstaller: Cardinal;
  Index: Integer;
begin
  Result := False;
  if not RegGetSubkeyNames(RootKey, UninstallKey, Subkeys) then
    exit;
  for Index := 0 to GetArrayLength(Subkeys) - 1 do
  begin
    Key := UninstallKey + '\' + Subkeys[Index];
    if IsProductCode(Subkeys[Index]) and
       RegQueryStringValue(RootKey, Key, 'DisplayName', DisplayName) and
       (DisplayName = DisplayNameToMatch) and
       RegQueryStringValue(RootKey, Key, 'Publisher', Publisher) and
       (Publisher = '{#ConflictingMsiPublisher}') and
       RegQueryDWordValue(RootKey, Key, 'WindowsInstaller', WindowsInstaller) and
       (WindowsInstaller = 1) then
    begin
      Result := True;
      exit;
    end;
  end;
end;

function HasOtherEditionInno(RootKey: Integer): Boolean;
var
  Key: String;
  DisplayName: String;
  Publisher: String;
begin
  Key := UninstallKey + '\{#OtherEditionInnoAppId}_is1';
  Result := RegQueryStringValue(RootKey, Key, 'DisplayName', DisplayName) and
    (DisplayName = '{#OtherEditionDisplayName}') and
    RegQueryStringValue(RootKey, Key, 'Publisher', Publisher) and
    (Publisher = '{#ConflictingMsiPublisher}');
end;

function HasOtherEditionRegistration(): Boolean;
begin
  Result :=
    HasMatchingMsiProduct(HKEY_LOCAL_MACHINE_64, '{#OtherEditionDisplayName}') or
    HasMatchingMsiProduct(HKEY_LOCAL_MACHINE_32, '{#OtherEditionDisplayName}') or
    HasMatchingMsiProduct(HKEY_CURRENT_USER_64, '{#OtherEditionDisplayName}') or
    HasMatchingMsiProduct(HKEY_CURRENT_USER_32, '{#OtherEditionDisplayName}') or
    HasOtherEditionInno(HKEY_LOCAL_MACHINE_64) or
    HasOtherEditionInno(HKEY_LOCAL_MACHINE_32) or
    HasOtherEditionInno(HKEY_CURRENT_USER_64) or
    HasOtherEditionInno(HKEY_CURRENT_USER_32);
end;

procedure AddMatchingMsiProducts(RootKey: Integer; var ProductCodes: TArrayOfString);
var
  Subkeys: TArrayOfString;
  Key: String;
  DisplayName: String;
  Publisher: String;
  WindowsInstaller: Cardinal;
  Index: Integer;
  Count: Integer;
  ExistingIndex: Integer;
  AlreadyPresent: Boolean;
begin
  { A missing uninstall key simply means this registry view has no product. }
  if not RegGetSubkeyNames(RootKey, UninstallKey, Subkeys) then
    exit;

  for Index := 0 to GetArrayLength(Subkeys) - 1 do
  begin
    Key := UninstallKey + '\' + Subkeys[Index];
    if IsProductCode(Subkeys[Index]) and
       RegQueryStringValue(RootKey, Key, 'DisplayName', DisplayName) and
       (DisplayName = '{#ConflictingMsiDisplayName}') and
       RegQueryStringValue(RootKey, Key, 'Publisher', Publisher) and
       (Publisher = '{#ConflictingMsiPublisher}') and
       RegQueryDWordValue(RootKey, Key, 'WindowsInstaller', WindowsInstaller) and
       (WindowsInstaller = 1) then
    begin
      AlreadyPresent := False;
      for ExistingIndex := 0 to GetArrayLength(ProductCodes) - 1 do
      begin
        if ProductCodes[ExistingIndex] = Subkeys[Index] then
          AlreadyPresent := True;
      end;
      if not AlreadyPresent then
      begin
        Count := GetArrayLength(ProductCodes);
        SetArrayLength(ProductCodes, Count + 1);
        ProductCodes[Count] := Subkeys[Index];
        Log('SD-300 matched same-edition MSI registration: ' + Subkeys[Index]);
      end;
    end;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ProductCodes: TArrayOfString;
  ProductCode: String;
  ExitCode: Integer;
  Args: String;
  Index: Integer;
begin
  Result := '';

  if HasOtherEditionRegistration() then
  begin
    Result := 'The other SD-300 edition is still registered. This installer ' +
      'stopped before changing anything because it cannot retire that scope ' +
      'safely. Uninstall the existing edition, or use the recommended managed ' +
      'installer from https://github.com/QubeTX/qube-system-diagnostics/releases/latest';
    exit;
  end;

  { Shutdown is deliberately independent from /PRESERVEGUISTATE: that switch
    preserves preferences during a channel transition, never a live process
    whose image is about to be replaced. StopExistingGui uses SW_HIDE and the
    CLI's authenticated lifecycle endpoint, and fails before file mutation. }
  StopExistingGui;

  AddMatchingMsiProducts(HKEY_LOCAL_MACHINE_64, ProductCodes);
  AddMatchingMsiProducts(HKEY_LOCAL_MACHINE_32, ProductCodes);
  AddMatchingMsiProducts(HKEY_CURRENT_USER_64, ProductCodes);
  AddMatchingMsiProducts(HKEY_CURRENT_USER_32, ProductCodes);

  if GetArrayLength(ProductCodes) > MaxConflictingMsiProducts then
  begin
    Result := 'More than 32 matching MSI registrations were found. ' +
      'The installer stopped before changing anything rather than guessing.';
    exit;
  end;

  for Index := 0 to GetArrayLength(ProductCodes) - 1 do
  begin
    ProductCode := ProductCodes[Index];
    Args := '/x "' + ProductCode + '" /qn /norestart SD300PRESERVEGUISTATE=1';
    Log('Removing same-edition MSI before changing the install channel: ' + ProductCode);
    if not Exec(ExpandConstant('{sys}\msiexec.exe'), Args, '', SW_HIDE,
        ewWaitUntilTerminated, ExitCode) then
    begin
      Result := 'Could not launch Windows Installer to remove the previous MSI. ' +
        'The existing installation was left unchanged.';
      exit;
    end;

    if (ExitCode = ErrorSuccessRebootInitiated) or
       (ExitCode = ErrorSuccessRebootRequired) then
    begin
      NeedsRestart := True;
      Result := 'Windows must restart to finish removing the previous MSI. ' +
        'Restart, then run this installer again.';
      exit;
    end;
    if (ExitCode <> ErrorSuccess) and (ExitCode <> ErrorUnknownProduct) then
    begin
      Result := 'The previous MSI could not be removed safely (exit ' +
        IntToStr(ExitCode) + '). It remains installed; this installer did not continue.';
      exit;
    end;
  end;
end;
