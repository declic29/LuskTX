# test-docx.ps1 — vérifie que Lusk Texte ouvre et réenregistre des fichiers Word sans perte.
#
# Usage :  powershell -ExecutionPolicy Bypass -File C:\lusk-texte\outils\test-docx.ps1 C:\docx-tests
#
# Pour chaque .docx du dossier :
#   1. Lusk Texte l'ouvre puis l'enregistre sous un nouveau nom (aller-retour) ;
#   2. le texte de l'original et de la copie est comparé : il doit être identique ;
#   3. le nombre de pages et de mots est relevé pour les deux ;
#   4. un PDF est produit, pour comparer à l'œil avec l'original ouvert dans Word.
# Les résultats vont dans <dossier>\_resultats (copies, PDF, différences, rapport.csv).

param([string]$Dossier = "C:\docx-tests")

$ErrorActionPreference = "Continue"
$racine = Split-Path -Parent $PSScriptRoot
$cli = Join-Path $racine "target\release\wordcraft-cli.exe"
if (-not (Test-Path $cli)) {
    Write-Host "Compilation de l'outil en ligne de commande (une seule fois)..."
    cargo build --release -p wordcraft-cli --manifest-path (Join-Path $racine "Cargo.toml")
    if (-not (Test-Path $cli)) { Write-Host "La compilation a échoué." -ForegroundColor Red; exit 1 }
}
if (-not (Test-Path $Dossier)) { Write-Host "Dossier introuvable : $Dossier" -ForegroundColor Red; exit 1 }

$sortie = Join-Path $Dossier "_resultats"
New-Item -ItemType Directory -Force $sortie | Out-Null
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

function Valeur($json, $cle) {
    if ($json -match ('"' + $cle + '"\s*:\s*(\d+)')) { return [int]$Matches[1] } else { return $null }
}

$rapport = @()
$fichiers = Get-ChildItem $Dossier -Filter *.docx -File | Where-Object { $_.Name -notlike '~$*' }
if (-not $fichiers) { Write-Host "Aucun fichier .docx dans $Dossier" -ForegroundColor Yellow; exit 0 }

foreach ($f in $fichiers) {
    $nom = $f.BaseName
    Write-Host "→ $($f.Name)"
    $copie = Join-Path $sortie "$nom.lusk.docx"
    $etat = "OK"; $detail = ""
    $p1 = $null; $p2 = $null; $m1 = $null; $m2 = $null

    $err = & $cli convert $f.FullName $copie 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path $copie)) {
        $etat = "ÉCHEC"; $detail = $err.Trim()
    } else {
        $t1 = (& $cli text $f.FullName 2>$null | Out-String)
        $t2 = (& $cli text $copie 2>$null | Out-String)
        if ($t1 -ne $t2) {
            $etat = "TEXTE MODIFIÉ"
            $diff = Compare-Object ($t1 -split "`r?`n") ($t2 -split "`r?`n")
            $diff | ForEach-Object { "$($_.SideIndicator) $($_.InputObject)" } |
                Set-Content -Encoding UTF8 (Join-Path $sortie "$nom.differences.txt")
            $detail = "$($diff.Count) ligne(s) différente(s), voir $nom.differences.txt (<= original, => copie)"
        }
        $i1 = & $cli info $f.FullName 2>$null | Out-String
        $i2 = & $cli info $copie 2>$null | Out-String
        $p1 = Valeur $i1 "pages"; $p2 = Valeur $i2 "pages"
        $m1 = Valeur $i1 "words"; $m2 = Valeur $i2 "words"
        if ($etat -eq "OK" -and ($p1 -ne $p2 -or $m1 -ne $m2)) { $etat = "MISE EN PAGE DIFFÉRENTE" }
        & $cli convert $f.FullName (Join-Path $sortie "$nom.lusk.pdf") 2>$null | Out-Null
    }
    $rapport += [pscustomobject]@{
        Fichier = $f.Name; Etat = $etat
        Pages = "$p1 → $p2"; Mots = "$m1 → $m2"; Detail = $detail
    }
}

Write-Host ""
$rapport | Format-Table Fichier, Etat, Pages, Mots -AutoSize
$rapport | Export-Csv (Join-Path $sortie "rapport.csv") -NoTypeInformation -Encoding UTF8 -Delimiter ";"
$ok = ($rapport | Where-Object Etat -eq "OK").Count
Write-Host "$ok / $($rapport.Count) fichier(s) sans perte. Détails : $sortie\rapport.csv"
