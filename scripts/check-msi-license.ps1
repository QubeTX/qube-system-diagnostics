param(
    [Parameter(Mandatory = $true)][string]$MsiPath
)

$ErrorActionPreference = 'Stop'
$expected = [IO.File]::ReadAllText((Join-Path $PSScriptRoot '..\wix\License.rtf'))
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.OpenDatabase([IO.Path]::GetFullPath($MsiPath), 0)
$view = $database.OpenView("SELECT ``Text`` FROM ``Control`` WHERE ``Dialog_`` = 'LicenseAgreementDlg' AND ``Control`` = 'LicenseText'")
try {
    $view.Execute()
    $record = $view.Fetch()
    if (-not $record) { throw "No LicenseAgreementDlg.LicenseText in $MsiPath" }
    $actual = $record.StringData(1)
    if ($actual.Replace("`r`n", "`n").Trim() -cne $expected.Replace("`r`n", "`n").Trim()) {
        throw "Embedded MSI license differs from wix/License.rtf: $MsiPath"
    }
    Add-Type -AssemblyName System.Windows.Forms
    $textBox = New-Object System.Windows.Forms.RichTextBox
    try {
        $textBox.Rtf = $actual
        if ($textBox.Text -notmatch 'PolyForm Noncommercial License 1\.0\.0' -or
            $textBox.Text -notmatch 'Required Notice: Copyright Emmett S' -or
            $textBox.Text -notmatch 'Control can be direct or indirect\.' -or
            $textBox.Text -match '(?i)lorem ipsum') {
            throw "Rendered MSI license is missing expected content or contains placeholder text: $MsiPath"
        }
        Write-Host "PASS: $MsiPath embeds the complete license; Windows RichEdit renders $($textBox.Text.Length) characters."
    } finally { $textBox.Dispose() }
} finally {
    $view.Close()
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($view)
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($database)
    [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($installer)
}
