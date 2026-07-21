[CmdletBinding()]
param(
    [Parameter()]
    [string]$BinaryPath = (Join-Path $PSScriptRoot '..\gui\zig-out\bin\sd300-gui.exe'),

    [Parameter()]
    [ValidateRange(0, 300)]
    [int]$WarmupSeconds = 4,

    [Parameter()]
    [ValidateRange(5, 7200)]
    [int]$DurationSeconds = 15,

    [Parameter()]
    [ValidateRange(100, 5000)]
    [int]$SampleIntervalMilliseconds = 250,

    [Parameter()]
    [ValidateSet('Overview', 'CPU', 'Memory', 'Disk', 'GPU', 'Network', 'Processes', 'Thermals', 'Drivers')]
    [string]$Section = 'Overview'
)

$ErrorActionPreference = 'Stop'
$resolvedBinary = (Resolve-Path -LiteralPath $BinaryPath).Path
$startedAt = Get-Date
$process = Start-Process -FilePath $resolvedBinary -WorkingDirectory (Split-Path -Parent $resolvedBinary) -PassThru

if (-not ('Sd300PerformanceNative' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;

public static class Sd300PerformanceNative {
    public delegate bool EnumProc(IntPtr hwnd, IntPtr context);

    [DllImport("user32.dll")]
    public static extern bool EnumChildWindows(IntPtr parent, EnumProc callback, IntPtr context);

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetClassName(IntPtr hwnd, StringBuilder text, int maximum);

    [DllImport("user32.dll")]
    public static extern IntPtr SendMessage(IntPtr hwnd, uint message, IntPtr wParam, IntPtr lParam);

    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr OpenEvent(uint desiredAccess, bool inheritHandle, string name);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool SetEvent(IntPtr handle);

    [DllImport("kernel32.dll")]
    public static extern bool CloseHandle(IntPtr handle);
}
'@
}

try {
    if ($Section -ne 'Overview') {
        $windowDeadline = (Get-Date).AddSeconds(10)
        do {
            Start-Sleep -Milliseconds 100
            $process.Refresh()
        } while ($process.MainWindowHandle -eq [IntPtr]::Zero -and -not $process.HasExited -and (Get-Date) -lt $windowDeadline)
        if ($process.MainWindowHandle -eq [IntPtr]::Zero) {
            throw 'The SD-300 GUI did not expose a foreground window for section selection.'
        }

        $script:sd300Surface = [IntPtr]::Zero
        $enumCallback = [Sd300PerformanceNative+EnumProc] {
            param($handle, $context)
            $className = [Text.StringBuilder]::new(128)
            [void][Sd300PerformanceNative]::GetClassName($handle, $className, 128)
            if ($className.ToString() -eq 'NativeSdkGpuSurface') {
                $script:sd300Surface = $handle
                return $false
            }
            return $true
        }
        [void][Sd300PerformanceNative]::EnumChildWindows(
            $process.MainWindowHandle,
            $enumCallback,
            [IntPtr]::Zero
        )
        if ($script:sd300Surface -eq [IntPtr]::Zero) {
            throw 'The SD-300 Native SDK canvas surface was not present.'
        }

        # Fixed chrome coordinates in the app's 1180 x 760 logical layout.
        # SendMessage exercises the same native pointer path without moving the
        # operator's mouse or launching a second automation transport.
        $sectionY = @{
            CPU = 188
            Memory = 222
            Disk = 256
            GPU = 290
            Network = 324
            Processes = 358
            Thermals = 392
            Drivers = 426
        }[$Section]
        $sectionX = 100
        $pointerCoordinates = [IntPtr](($sectionY -shl 16) -bor ($sectionX -band 0xffff))
        [void][Sd300PerformanceNative]::SendMessage($script:sd300Surface, 0x0201, [IntPtr]1, $pointerCoordinates)
        [void][Sd300PerformanceNative]::SendMessage($script:sd300Surface, 0x0202, [IntPtr]0, $pointerCoordinates)
    }

    if ($WarmupSeconds -gt 0) {
        Start-Sleep -Seconds $WarmupSeconds
    }

    $process.Refresh()
    if ($process.HasExited) {
        throw "SD-300 GUI exited during warmup with code $($process.ExitCode)."
    }

    $samples = [System.Collections.Generic.List[double]]::new()
    $workingSets = [System.Collections.Generic.List[long]]::new()
    $privateBytes = [System.Collections.Generic.List[long]]::new()
    $clock = [System.Diagnostics.Stopwatch]::StartNew()
    $previousWallSeconds = $clock.Elapsed.TotalSeconds
    $previousCpuSeconds = $process.TotalProcessorTime.TotalSeconds
    $initialCpuSeconds = $previousCpuSeconds

    while ($clock.Elapsed.TotalSeconds -lt $DurationSeconds) {
        Start-Sleep -Milliseconds $SampleIntervalMilliseconds
        $process.Refresh()
        if ($process.HasExited) {
            throw "SD-300 GUI exited during measurement with code $($process.ExitCode)."
        }

        $wallSeconds = $clock.Elapsed.TotalSeconds
        $cpuSeconds = $process.TotalProcessorTime.TotalSeconds
        $wallDelta = $wallSeconds - $previousWallSeconds
        $cpuDelta = $cpuSeconds - $previousCpuSeconds
        if ($wallDelta -gt 0) {
            # 100% means one logical core was busy for the complete interval.
            $samples.Add(($cpuDelta / $wallDelta) * 100.0)
        }
        $workingSets.Add($process.WorkingSet64)
        $privateBytes.Add($process.PrivateMemorySize64)
        $previousWallSeconds = $wallSeconds
        $previousCpuSeconds = $cpuSeconds
    }

    $clock.Stop()
    $process.Refresh()
    $overallCpuPercent = (($process.TotalProcessorTime.TotalSeconds - $initialCpuSeconds) / $clock.Elapsed.TotalSeconds) * 100.0
    $sorted = @($samples | Sort-Object)
    $p95Index = [Math]::Max(0, [Math]::Ceiling($sorted.Count * 0.95) - 1)

    [ordered]@{
        schemaVersion = 1
        binary = $resolvedBinary
        section = $Section
        startedAt = $startedAt.ToString('o')
        warmupSeconds = $WarmupSeconds
        durationSeconds = [Math]::Round($clock.Elapsed.TotalSeconds, 3)
        sampleIntervalMilliseconds = $SampleIntervalMilliseconds
        sampleCount = $samples.Count
        cpuPercentOfOneLogicalCore = [ordered]@{
            average = [Math]::Round($overallCpuPercent, 2)
            p95 = [Math]::Round($sorted[$p95Index], 2)
            maximum = [Math]::Round(($sorted | Measure-Object -Maximum).Maximum, 2)
        }
        memoryMiB = [ordered]@{
            workingSetAverage = [Math]::Round((($workingSets | Measure-Object -Average).Average / 1MB), 2)
            workingSetMaximum = [Math]::Round((($workingSets | Measure-Object -Maximum).Maximum / 1MB), 2)
            privateAverage = [Math]::Round((($privateBytes | Measure-Object -Average).Average / 1MB), 2)
            privateMaximum = [Math]::Round((($privateBytes | Measure-Object -Maximum).Maximum / 1MB), 2)
        }
    } | ConvertTo-Json -Depth 4
}
finally {
    if ($null -ne $process -and -not $process.HasExited) {
        $eventHandle = [Sd300PerformanceNative]::OpenEvent(0x0002, $false, 'Local\SD300.Gui.Quit.v1')
        $exitRequested = $false
        if ($eventHandle -ne [IntPtr]::Zero) {
            try {
                $exitRequested = [Sd300PerformanceNative]::SetEvent($eventHandle)
            }
            finally {
                [void][Sd300PerformanceNative]::CloseHandle($eventHandle)
            }
        }
        if ($exitRequested) {
            [void]$process.WaitForExit(10000)
        }
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
            [void]$process.WaitForExit(5000)
            throw 'The SD-300 GUI did not exit through its authenticated lifecycle endpoint after performance measurement.'
        }
    }
}
