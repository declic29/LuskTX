# test-epub.ps1 — valide les EPUB de Lusk Texte avec EPUBCheck, l'outil de validation officiel (W3C),
# et leur accessibilité avec Ace (consortium DAISY).
#
# Usage :
#   powershell -ExecutionPolicy Bypass -File C:\lusk-texte\outils\test-epub.ps1
#       → fabrique un livre complet (pages liminaires, parties, chapitres, notes, listes, tableau,
#         image, lien) et un document vide, les exporte en EPUB et les fait valider (test
#         book::epub::tests::epubcheck_finds_nothing_wrong) ;
#   powershell -ExecutionPolicy Bypass -File C:\lusk-texte\outils\test-epub.ps1 C:\mes-livres
#       → valide chaque .epub du dossier (ou le fichier .epub donné).
#
# EPUBCheck (licence BSD) est téléchargé une seule fois depuis GitHub dans outils\epubcheck.
# Il faut Java 11 ou plus récent (par exemple Eclipse Temurin : https://adoptium.net).
# Ace (licence MIT) est installé une seule fois dans outils\ace si Node.js est présent
# (https://nodejs.org) ; sans Node.js, la vérification d'accessibilité est sautée.

param([string]$Chemin = "")

$ErrorActionPreference = "Stop"
$version = "5.2.1"
$racine = Split-Path -Parent $PSScriptRoot
$dossier = Join-Path $PSScriptRoot "epubcheck"
$jar = Join-Path $dossier "epubcheck-$version\epubcheck.jar"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if (-not (Get-Command java -ErrorAction SilentlyContinue)) {
    Write-Host "Java est introuvable. Installez Java 11 ou plus récent (https://adoptium.net) puis relancez." -ForegroundColor Red
    exit 1
}

if (-not (Test-Path $jar)) {
    Write-Host "Téléchargement d'EPUBCheck $version (une seule fois)..."
    New-Item -ItemType Directory -Force $dossier | Out-Null
    $zip = Join-Path $dossier "epubcheck-$version.zip"
    $url = "https://github.com/w3c/epubcheck/releases/download/v$version/epubcheck-$version.zip"
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
    Expand-Archive -Path $zip -DestinationPath $dossier -Force
    Remove-Item $zip
    if (-not (Test-Path $jar)) { Write-Host "EPUBCheck n'a pas pu être installé." -ForegroundColor Red; exit 1 }
}

# Ace : installé une fois dans outils\ace quand Node.js est là.
$aceDossier = Join-Path $PSScriptRoot "ace"
$ace = Join-Path $aceDossier "node_modules\.bin\ace.cmd"
if (Get-Command npm -ErrorAction SilentlyContinue) {
    if (-not (Test-Path $ace)) {
        Write-Host "Installation d'Ace (accessibilité, une seule fois)..."
        New-Item -ItemType Directory -Force $aceDossier | Out-Null
        $ErrorActionPreference = "Continue"
        & npm install --prefix $aceDossier --no-audit --no-fund "@daisy/ace" | Out-Null
        $ErrorActionPreference = "Stop"
    }
    if (-not (Test-Path $ace)) { Write-Host "Ace n'a pas pu être installé : accessibilité non vérifiée." -ForegroundColor Yellow; $ace = $null }
} else {
    Write-Host "Node.js est introuvable : l'accessibilité ne sera pas vérifiée (https://nodejs.org)." -ForegroundColor Yellow
    $ace = $null
}

if ($Chemin -eq "") {
    Write-Host "Export d'un livre de test en EPUB et validation par EPUBCheck$(if ($ace) { ' et Ace' })..."
    $env:EPUBCHECK_JAR = $jar
    if ($ace) { $env:ACE_BIN = $ace }
    cargo test -p wordcraft-engine --lib --manifest-path (Join-Path $racine "Cargo.toml") -- epubcheck_finds_nothing_wrong --nocapture
    if ($LASTEXITCODE -eq 0) {
        Write-Host "EPUBCheck : aucune erreur, aucun avertissement.$(if ($ace) { ' Ace : accessibilité conforme.' })" -ForegroundColor Green
    } else {
        Write-Host "EPUBCheck a trouvé des problèmes (voir ci-dessus)." -ForegroundColor Red
    }
    exit $LASTEXITCODE
}

if (-not (Test-Path $Chemin)) { Write-Host "Introuvable : $Chemin" -ForegroundColor Red; exit 1 }
$fichiers = if ((Get-Item $Chemin).PSIsContainer) { Get-ChildItem $Chemin -Filter *.epub -File } else { Get-Item $Chemin }
if (-not $fichiers) { Write-Host "Aucun fichier .epub dans $Chemin" -ForegroundColor Yellow; exit 0 }
$echecs = 0
foreach ($f in $fichiers) {
    Write-Host "→ $($f.Name)"
    $ErrorActionPreference = "Continue"
    & java -jar $jar $f.FullName 2>&1 | ForEach-Object { "   $_" } | Where-Object { $_ -notmatch "JAVA_TOOL_OPTIONS" }
    if ($LASTEXITCODE -ne 0) { $echecs++ }
    if ($ace) {
        $rapport = Join-Path $f.DirectoryName ("ace-" + $f.BaseName)
        & $ace -f -s -o $rapport $f.FullName 2>&1 | Out-Null
        $json = Join-Path $rapport "report.json"
        if ((Test-Path $json) -and ((Get-Content $json -Raw -Encoding UTF8 | ConvertFrom-Json).'earl:result'.'earl:outcome' -eq 'pass')) {
            Write-Host "   Accessibilité (Ace) : conforme."
        } else {
            Write-Host "   Accessibilité (Ace) : problèmes, voir $rapport\report.html" -ForegroundColor Red
            $echecs++
        }
    }
}
Write-Host ""
if ($echecs -eq 0) {
    Write-Host "$($fichiers.Count) fichier(s) EPUB valide(s)." -ForegroundColor Green
} else {
    Write-Host "$echecs fichier(s) sur $($fichiers.Count) avec des erreurs." -ForegroundColor Red
}
