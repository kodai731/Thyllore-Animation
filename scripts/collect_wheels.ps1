[CmdletBinding()]
param(
    [string]$WheelsDir = "blender_addon/wheels",
    [switch]$SkipMaturin,
    # Build only this crate (default: every wheel crate)
    [string]$Crate = "",
    # Build with the debug-log feature (crates that declare it)
    [switch]$DebugLog
)

$ErrorActionPreference = "Stop"
$RepoRoot = Resolve-Path "$PSScriptRoot/.."
$AbsWheels = Join-Path $RepoRoot $WheelsDir

New-Item -ItemType Directory -Path $AbsWheels -Force | Out-Null

function Test-CrateDeclaresDebugLog([string]$CrateName) {
    $manifest = Get-Content (Join-Path $RepoRoot "crates/$CrateName/Cargo.toml") -Raw
    return $manifest -match '(?m)^debug-log\s*='
}

if (-not $SkipMaturin) {
    $PrevErrorAction = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        & python -m pip install --quiet maturin *>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "maturin install failed"
        }
    } finally {
        $ErrorActionPreference = $PrevErrorAction
    }

    $Crates = if ($Crate) { @($Crate) } else { @("thyllore-ml-core", "thyllore-avatar-api") }

    foreach ($crateName in $Crates) {
        if ($DebugLog) {
            if (-not (Test-CrateDeclaresDebugLog $crateName)) {
                throw "-Crate $crateName cannot be combined with -DebugLog (no debug-log feature in its Cargo.toml)"
            }
            $Features = "python,debug-log"
            Write-Host "[collect_wheels] Building $crateName DEBUG wheel (features: $Features)" -ForegroundColor Cyan
        } else {
            $Features = "python"
            Write-Host "[collect_wheels] Building $crateName wheel via maturin..." -ForegroundColor Cyan
        }
        $PrevErrorAction = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        try {
            Push-Location (Join-Path $RepoRoot "crates/$crateName")
            $PrevRustFlags = $env:RUSTFLAGS
            try {
                $RemapFlag = "--remap-path-prefix=$HOME=."
                $env:RUSTFLAGS = if ($PrevRustFlags) { "$PrevRustFlags $RemapFlag" } else { $RemapFlag }
                & maturin build --release --features $Features --out $AbsWheels *>&1 | Out-Null
                if ($LASTEXITCODE -ne 0) {
                    throw "maturin build failed"
                }
            } finally {
                $env:RUSTFLAGS = $PrevRustFlags
                Pop-Location
            }
        } finally {
            $ErrorActionPreference = $PrevErrorAction
        }
    }
}

# SHA256 manifest written for local debugging only — gitignored.
Write-Host "[collect_wheels] Writing local SHA256 manifest (gitignored)..." -ForegroundColor Cyan
$HashesFile = Join-Path $AbsWheels "HASHES.txt"
$Lines = New-Object System.Collections.Generic.List[string]
foreach ($wheel in Get-ChildItem $AbsWheels -Filter "*.whl" | Sort-Object Name) {
    $hash = (Get-FileHash $wheel.FullName -Algorithm SHA256).Hash.ToLower()
    $Lines.Add("$($wheel.Name)  $hash")
}
[System.IO.File]::WriteAllText($HashesFile, ($Lines -join "`n") + "`n")

$WheelCount = (Get-ChildItem $AbsWheels -Filter "*.whl").Count
Write-Host "[collect_wheels] Done. $WheelCount wheels in $AbsWheels"
