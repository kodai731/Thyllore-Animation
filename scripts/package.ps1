param(
    [switch]$SkipBuild,
    [switch]$NoMl
)

$ErrorActionPreference = "Stop"

$version = (Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"(.+)"' | Select-Object -First 1 | ForEach-Object { $_.Matches[0].Groups[1].Value })
$packageName = "thyllore-animation-v$version-win-x64"
$outDir = "dist/$packageName"
$ortDir = "vendor/onnxruntime/onnxruntime-win-x64-1.23.2/lib"

Write-Host "Packaging $packageName ..." -ForegroundColor Cyan

if (-not $SkipBuild) {
    Write-Host "Building release..." -ForegroundColor Yellow
    if ($NoMl) {
        cargo build --release --no-default-features
    } else {
        cargo build --release
    }
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Build failed"
        exit 1
    }
}

$exe = "target/release/thyllore-animation.exe"
if (-not (Test-Path $exe)) {
    Write-Error "Release binary not found: $exe"
    exit 1
}

if (Test-Path $outDir) {
    Remove-Item -Recurse -Force $outDir
}

New-Item -ItemType Directory -Path $outDir -Force | Out-Null
New-Item -ItemType Directory -Path "$outDir/assets/shaders" -Force | Out-Null
New-Item -ItemType Directory -Path "$outDir/assets/fonts" -Force | Out-Null
New-Item -ItemType Directory -Path "$outDir/assets/textures" -Force | Out-Null
New-Item -ItemType Directory -Path "$outDir/log" -Force | Out-Null

Copy-Item $exe "$outDir/"

if (-not $NoMl) {
    if (Test-Path "$ortDir/onnxruntime.dll") {
        Copy-Item "$ortDir/onnxruntime.dll" "$outDir/"
        Copy-Item "$ortDir/onnxruntime_providers_shared.dll" "$outDir/"
    } else {
        Write-Warning "ONNX Runtime DLLs not found at $ortDir"
    }

    if (Test-Path "ml/model/curve_copilot.onnx") {
        New-Item -ItemType Directory -Path "$outDir/ml/model" -Force | Out-Null
        Copy-Item "ml/model/curve_copilot.onnx" "$outDir/ml/model/"
    } else {
        Write-Warning "ML model not found at ml/model/curve_copilot.onnx"
    }
}

Copy-Item "assets/shaders/*.spv" "$outDir/assets/shaders/"

Copy-Item "assets/fonts/Roboto-Regular.ttf" "$outDir/assets/fonts/"
Copy-Item "assets/fonts/mplus-1p-regular.ttf" "$outDir/assets/fonts/"
Copy-Item "assets/fonts/Dokdo-Regular.ttf" "$outDir/assets/fonts/"
Copy-Item "assets/fonts/LICENSE-Roboto.txt" "$outDir/assets/fonts/"
Copy-Item "assets/fonts/LICENSE-Dokdo.txt" "$outDir/assets/fonts/"

Copy-Item "assets/textures/lightIcon.png" "$outDir/assets/textures/"
Copy-Item "assets/textures/white.png" "$outDir/assets/textures/"

Copy-Item "LICENSE" "$outDir/"
Copy-Item "THIRD_PARTY_LICENSES.md" "$outDir/"
Copy-Item "README.md" "$outDir/"

$zipPath = "dist/$packageName.zip"
if (Test-Path $zipPath) {
    Remove-Item $zipPath
}
Compress-Archive -Path "$outDir/*" -DestinationPath $zipPath

$zipSize = [math]::Round((Get-Item $zipPath).Length / 1MB, 1)
Write-Host ""
Write-Host "Package created: $zipPath ($zipSize MB)" -ForegroundColor Green
Write-Host ""
Write-Host "Contents:" -ForegroundColor Cyan
Get-ChildItem -Recurse $outDir | ForEach-Object {
    $rel = $_.FullName.Replace((Resolve-Path $outDir).Path, "").TrimStart("\")
    if ($_.PSIsContainer) { Write-Host "  $rel/" } else { Write-Host "  $rel" }
}
