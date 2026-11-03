# Makes .prcraft documents open in PresentCraft when double-clicked, and adds
# PresentCraft to "Open with" for .iapresenter and Markdown files.
# Per-user (HKCU), so no administrator rights are needed. Run it from the
# folder that holds PresentCraft.exe, after moving that folder where it
# will stay:
#
#   powershell -ExecutionPolicy Bypass -File register-file-types.ps1
#
# Undo with: powershell -ExecutionPolicy Bypass -File register-file-types.ps1 -Remove
param([switch]$Remove)
$ErrorActionPreference = "Stop"

$exe = Join-Path $PSScriptRoot "PresentCraft.exe"
$classes = "HKCU:\Software\Classes"
$progId = "PresentCraft.Presentation"
$appKey = "$classes\Applications\PresentCraft.exe"

function Set-Default($path, $value) {
    New-Item -Path $path -Force | Out-Null
    Set-Item -Path $path -Value $value
}

if ($Remove) {
    foreach ($k in "$classes\$progId", $appKey) {
        if (Test-Path $k) { Remove-Item -Path $k -Recurse -Force }
    }
    $ext = "$classes\.prcraft"
    if ((Test-Path $ext) -and ((Get-Item $ext).GetValue("") -eq $progId)) {
        Remove-Item -Path $ext -Recurse -Force
    }
    foreach ($e in ".iapresenter", ".md", ".markdown") {
        $k = "$classes\$e\OpenWithProgids"
        if (Test-Path $k) { Remove-ItemProperty -Path $k -Name $progId -ErrorAction SilentlyContinue }
    }
} else {
    if (-not (Test-Path $exe)) { throw "PresentCraft.exe not found next to this script" }
    $open = "`"$exe`" `"%1`""
    Set-Default "$classes\$progId" "PresentCraft Presentation"
    Set-Default "$classes\$progId\DefaultIcon" "`"$exe`",0"
    Set-Default "$classes\$progId\shell\open\command" $open
    Set-Default "$classes\.prcraft" $progId
    New-ItemProperty -Path "$classes\.prcraft" -Name "Content Type" -Value "application/x-presentcraft" -Force | Out-Null
    foreach ($e in ".iapresenter", ".md", ".markdown") {
        New-Item -Path "$classes\$e\OpenWithProgids" -Force | Out-Null
        New-ItemProperty -Path "$classes\$e\OpenWithProgids" -Name $progId -Value "" -PropertyType String -Force | Out-Null
    }
    Set-Default "$appKey\shell\open\command" $open
    New-ItemProperty -Path $appKey -Name "FriendlyAppName" -Value "PresentCraft" -Force | Out-Null
}

# Tell Explorer the associations changed so icons refresh.
Add-Type -Namespace PresentCraft -Name Shell -MemberDefinition @"
[System.Runtime.InteropServices.DllImport("shell32.dll")]
public static extern void SHChangeNotify(int eventId, uint flags, System.IntPtr a, System.IntPtr b);
"@
[PresentCraft.Shell]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
if ($Remove) { Write-Host "PresentCraft file types removed." } else { Write-Host "PresentCraft now opens .prcraft files." }
