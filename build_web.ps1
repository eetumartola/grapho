$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$distDir = Join-Path $repoRoot "dist"
$indexSource = Join-Path $repoRoot "crates\app\index.html"
$wasmOut = Join-Path $repoRoot "target\wasm32-unknown-unknown\release\grapho_web.wasm"
$fallbackWasmOut = Join-Path $repoRoot "target\wasm32-unknown-unknown\release\Grapho.wasm"

function Require-Command {
    param([string]$Name, [string]$Hint)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Error "Missing '$Name'. $Hint"
    }
}

Write-Host "Building wasm (release)..." -ForegroundColor Cyan

$installedTargets = & rustup target list --installed 2>$null
if ($LASTEXITCODE -ne 0) {
    Write-Warning "Could not query rustup targets. Ensure wasm32-unknown-unknown is installed."
} elseif (-not ($installedTargets -contains "wasm32-unknown-unknown")) {
    Write-Error "Missing target wasm32-unknown-unknown. Run: rustup target add wasm32-unknown-unknown"
}

& cargo build -p grapho --target wasm32-unknown-unknown --release --lib

Require-Command "wasm-bindgen" "Install it with: cargo install wasm-bindgen-cli"

if (-not (Test-Path $wasmOut)) {
    if (Test-Path $fallbackWasmOut) {
        $wasmOut = $fallbackWasmOut
    } else {
        Write-Error "Wasm output not found at $wasmOut or $fallbackWasmOut"
    }
}

if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir | Out-Null
}

Write-Host "Generating JS glue..." -ForegroundColor Cyan
& wasm-bindgen $wasmOut --target web --out-dir $distDir --out-name grapho

function Rename-ToLower {
    param([string]$Path)
    if (Test-Path $Path) {
        $dir = Split-Path $Path -Parent
        $name = Split-Path $Path -Leaf
        $lower = $name.ToLowerInvariant()
        if ($name -ne $lower) {
            $temp = Join-Path $dir ("_tmp_" + $lower)
            Move-Item -Path $Path -Destination $temp -Force
            Move-Item -Path $temp -Destination (Join-Path $dir $lower) -Force
        }
    }
}

Rename-ToLower (Join-Path $distDir "Grapho.js")
Rename-ToLower (Join-Path $distDir "Grapho.d.ts")
Rename-ToLower (Join-Path $distDir "Grapho_bg.wasm")
Rename-ToLower (Join-Path $distDir "Grapho_bg.wasm.d.ts")

if (-not (Test-Path $indexSource)) {
    Write-Error "index.html not found at $indexSource"
}

Copy-Item -Path $indexSource -Destination (Join-Path $distDir "index.html") -Force

Write-Host "Done. Serve the dist/ folder with any static web server." -ForegroundColor Green
