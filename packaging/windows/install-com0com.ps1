param([string]$LogPath)

$ErrorActionPreference = 'Stop'
$Temp = Join-Path $env:TEMP 'casomira-com0com'
$Log = if ($LogPath) { $LogPath } else { Join-Path $env:TEMP 'Casomira-com0com-install.log' }

function Write-Log([string]$Message) {
    $Line = "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') $Message"
    Add-Content -Path $Log -Value $Line -Encoding UTF8
}

try {
    Remove-Item -Force $Log -ErrorAction SilentlyContinue
    Write-Log "Začíná instalace com0com. PowerShell=$($PSVersionTable.PSVersion), 64bitOS=$([Environment]::Is64BitOperatingSystem), 64bitProcess=$([Environment]::Is64BitProcess), TEMP=$env:TEMP"
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

    $Url = 'https://downloads.sourceforge.net/project/com0com/com0com/3.0.0.0/com0com-3.0.0.0-i386-and-x64-signed.zip'
    $ExpectedSha256 = '6e5d4359865277430d4ae88c73fb7e648a0ed8e81aea5002478179cfcb0bb0e1'
    $Zip = Join-Path $Temp 'com0com.zip'
    $Extract = Join-Path $Temp 'unpacked'
    New-Item -ItemType Directory -Force -Path $Temp | Out-Null
    $Curl = (Get-Command 'curl.exe' -ErrorAction SilentlyContinue).Source
    if (-not $Curl) { throw 'Ve Windows není dostupný curl.exe pro bezpečné stažení s následováním přesměrování.' }
    $DownloadOutput = & $Curl --fail --location --silent --show-error --retry 3 --retry-all-errors --output $Zip $Url 2>&1 | Out-String
    $DownloadExitCode = $LASTEXITCODE
    if ($DownloadExitCode -ne 0) {
        throw "Stažení z SourceForge selhalo (curl kód $DownloadExitCode): $DownloadOutput"
    }
    if (-not (Test-Path $Zip)) { throw 'curl.exe skončil bez chyby, ale archiv nebyl vytvořen.' }
    $ActualSha256 = (Get-FileHash -Path $Zip -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($ActualSha256 -ne $ExpectedSha256) {
        $FileLength = (Get-Item $Zip).Length
        $Bytes = [System.IO.File]::ReadAllBytes($Zip)
        $MagicLength = [Math]::Min(15, $Bytes.Length)
        $FileStart = if ($MagicLength -gt 0) { [Text.Encoding]::ASCII.GetString($Bytes, 0, $MagicLength) } else { '<prázdný soubor>' }
        Write-Log "Stažený obsah: délka=$FileLength bajtů, začátek='$FileStart'."
        throw "Nesouhlasí SHA-256 archivu. Očekáváno $ExpectedSha256, získáno $ActualSha256."
    }
    Write-Log 'Stažený archiv odpovídá ověřenému SHA-256.'

    Remove-Item -Recurse -Force $Extract -ErrorAction SilentlyContinue
    Expand-Archive -Path $Zip -DestinationPath $Extract -Force
    $Setup = Join-Path $Extract 'Setup_com0com_v3.0.0.0_W7_x64_signed.exe'
    if (-not (Test-Path $Setup)) { throw 'V archivu chybí 64bitový instalátor com0com.' }

    # NSIS /S installs silently. This installer is already running elevated.
    $Process = Start-Process -FilePath $Setup -ArgumentList '/S' -Wait -PassThru
    Write-Log "Instalátor ovladače skončil kódem $($Process.ExitCode)."
    if ($Process.ExitCode -ne 0) { throw "Instalátor com0com vrátil kód $($Process.ExitCode)." }

    $SetupcCandidates = @(
        (Join-Path $env:ProgramFiles 'com0com/setupc.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'com0com/setupc.exe'),
        (Join-Path $env:ProgramFiles 'com0com/setupc'),
        (Join-Path ${env:ProgramFiles(x86)} 'com0com/setupc')
    )
    $Setupc = $SetupcCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $Setupc) { throw 'Instalátor doběhl, ale setupc.exe nebyl nalezen ve standardních složkách com0com.' }
    Write-Log "Nalezen setupc: $Setupc"

    # The silent installer can return success while leaving the root device
    # present but unconfigured (PnP problem code 1). Explicitly stage/install
    # its INF and rescan before asking setupc to publish COM-class endpoints.
    $Com0comInf = Join-Path (Split-Path -Parent $Setupc) 'com0com.inf'
    if (-not (Test-Path $Com0comInf)) { throw "Chybí INF ovladače com0com: $Com0comInf" }
    $NativeSystemDirectory = Join-Path $env:WINDIR 'System32'
    if ([Environment]::Is64BitOperatingSystem -and -not [Environment]::Is64BitProcess) {
        # A 32-bit PowerShell process is redirected away from System32. Sysnative
        # is the documented WOW64 alias for reaching the native 64-bit directory.
        $NativeSystemDirectory = Join-Path $env:WINDIR 'Sysnative'
    }
    $PnPUtil = Join-Path $NativeSystemDirectory 'pnputil.exe'
    if (-not (Test-Path $PnPUtil)) { throw "Nástroj PnPUtil nebyl nalezen: $PnPUtil" }
    Write-Log "Použije se PnPUtil: $PnPUtil"
    $PnpOutput = & $PnPUtil /add-driver $Com0comInf /install 2>&1 | Out-String
    $PnpExitCode = $LASTEXITCODE
    Write-Log "pnputil /add-driver com0com.inf /install skončil kódem $PnpExitCode.`n$PnpOutput"
    if ($PnpExitCode -ne 0) {
        $PnpExitHex = '{0:X8}' -f [int]$PnpExitCode
        if ([int]$PnpExitCode -eq -2146762480) {
            throw "Windows odmítl certifikát ovladače com0com (0x$PnpExitHex, CERT_E_WRONG_USAGE). Tento instalátor obsahuje starý cross-signed ovladač, který aktuální zásady zabezpečení Windows nepřijímají. Pár COM19/COM20 proto nebyl vytvořen. Je potřeba nová verze ovladače podepsaná přes Microsoft Hardware Dev Center; opakování instalace ani změna čísel COM portů to neopraví.`n$PnpOutput"
        }
        throw "Windows odmítl přidat ovladač com0com (pnputil kód $PnpExitCode / 0x$PnpExitHex): $PnpOutput"
    }

    $ScanOutput = & $PnPUtil /scan-devices 2>&1 | Out-String
    $ScanExitCode = $LASTEXITCODE
    Write-Log "pnputil /scan-devices skončil kódem $ScanExitCode.`n$ScanOutput"
    $RootDevice = $null
    for ($Attempt = 0; $Attempt -lt 20; $Attempt++) {
        $RootDevice = Get-CimInstance -ClassName Win32_PnPEntity -ErrorAction Stop |
            Where-Object { $_.PNPDeviceID -match '(?i)^ROOT\\COM0COM\\' } |
            Select-Object -First 1
        if ($RootDevice -and $RootDevice.ConfigManagerErrorCode -eq 0) { break }
        Start-Sleep -Seconds 1
    }
    if (-not $RootDevice) { throw 'Windows po přidání INF nenašel zařízení ROOT\COM0COM.' }
    Write-Log "Stav kořenového zařízení po instalaci INF: PNPDeviceID=$($RootDevice.PNPDeviceID), ConfigManagerErrorCode=$($RootDevice.ConfigManagerErrorCode), Name=$($RootDevice.Name)"
    if ($RootDevice.ConfigManagerErrorCode -ne 0) {
        try {
            $SetupApiTail = Get-Content (Join-Path $env:WINDIR 'INF/setupapi.dev.log') -Tail 100 -ErrorAction Stop | Out-String
            Write-Log "Konec Windows SetupAPI logu:`n$SetupApiTail"
        } catch {
            Write-Log "SetupAPI log se nepodařilo načíst: $($_.Exception.Message)"
        }
        throw "Windows nenačetl ovladač com0com (PnP kód $($RootDevice.ConfigManagerErrorCode)). Podrobnosti jsou v tomto instalačním logu."
    }

    # setupc opens com0com.inf and companion INF files relative to its current
    # directory. NSIS starts this script in a disposable temp folder.
    Push-Location (Split-Path -Parent $Setupc)
    try {
    $List = & $Setupc list 2>&1 | Out-String
    Write-Log "Aktuální konfigurace setupc list:`n$List"

    # Explicit PortName=COM19/COM20 creates com0com endpoints, but may leave
    # them outside Windows' standard Ports class. Third-party serial libraries
    # enumerate standard Ports-class devices, so ask the Ports class installer
    # to publish both endpoints and then assign the stable desired numbers.
    $Port19Line = $List -split "`r?`n" | Where-Object { $_ -match '(?i)^\s*CNCA\d+\s+.*(?:PortName=COM19|RealPortName=COM19)(?:,|\s|$)' } | Select-Object -First 1
    $Port20Line = $List -split "`r?`n" | Where-Object { $_ -match '(?i)^\s*CNCB\d+\s+.*(?:PortName=COM20|RealPortName=COM20)(?:,|\s|$)' } | Select-Object -First 1

    if ($Port19Line -and $Port20Line) {
        $Port19Name = [regex]::Match($Port19Line, '(?i)^\s*(CNCA\d+)').Groups[1].Value
        $Port20Name = [regex]::Match($Port20Line, '(?i)^\s*(CNCB\d+)').Groups[1].Value
        if (($Port19Name -replace '^CNCA', '') -ne ($Port20Name -replace '^CNCB', '')) {
            throw 'COM19 a COM20 nejsou v setupc přiřazeny ke stejnému páru.'
        }
    } else {
        # Reuse a configured COM# pair (including the pair the silent driver
        # installer created). Creating another pair on every retry is harmful.
        $CandidateA = $List -split "`r?`n" | Where-Object { $_ -match '(?i)^\s*CNCA\d+\s+.*PortName=COM#' } | Select-Object -First 1
        if ($CandidateA) {
            $PairIndex = [regex]::Match($CandidateA, '(?i)^\s*CNCA(\d+)').Groups[1].Value
            $Port19Line = $CandidateA
            $Port20Line = $List -split "`r?`n" | Where-Object { $_ -match "(?i)^\s*CNCB$PairIndex\s+.*PortName=COM#" } | Select-Object -First 1
            if (-not $Port20Line) { throw "Pár CNCA$PairIndex nemá odpovídající CNCB${PairIndex}:`n$List" }
            Write-Log "Použije se již existující pár CNCA$PairIndex/CNCB$PairIndex."
        } else {
            $PairOutput = & $Setupc install 'PortName=COM#' 'PortName=COM#' 2>&1 | Out-String
            $PairExitCode = $LASTEXITCODE
            Write-Log "setupc install Ports-class pair skončil kódem $PairExitCode.`n$PairOutput"
            if ($PairExitCode -ne 0) { throw "setupc install selhal (kód $PairExitCode): $PairOutput" }

            $List = & $Setupc list 2>&1 | Out-String
            $Port19Line = $List -split "`r?`n" | Where-Object { $_ -match '(?i)^\s*CNCA\d+\s+.*PortName=COM#' } | Select-Object -Last 1
            if (-not $Port19Line) { throw "setupc po instalaci nevrátil konec A páru:`n$List" }
            $PairIndex = [regex]::Match($Port19Line, '(?i)^\s*CNCA(\d+)').Groups[1].Value
            $Port20Line = $List -split "`r?`n" | Where-Object { $_ -match "(?i)^\s*CNCB$PairIndex\s+.*PortName=COM#" } | Select-Object -First 1
            if (-not $Port20Line) { throw "setupc po instalaci nevrátil konec B páru CNCA${PairIndex}:`n$List" }
        }
        $Port19Name = [regex]::Match($Port19Line, '(?i)^\s*(CNCA\d+)').Groups[1].Value
        $Port20Name = [regex]::Match($Port20Line, '(?i)^\s*(CNCB\d+)').Groups[1].Value
    }

    $Changes = @(
        [pscustomobject]@{ Device = $Port19Name; Setting = 'PortName=COM#' },
        [pscustomobject]@{ Device = $Port20Name; Setting = 'PortName=COM#' },
        [pscustomobject]@{ Device = $Port19Name; Setting = 'RealPortName=COM19' },
        [pscustomobject]@{ Device = $Port20Name; Setting = 'RealPortName=COM20' }
    )
    foreach ($Change in $Changes) {
        $ChangeOutput = & $Setupc change $Change.Device $Change.Setting 2>&1 | Out-String
        $ChangeExitCode = $LASTEXITCODE
        Write-Log "setupc change $($Change.Device) $($Change.Setting) skončil kódem $ChangeExitCode.`n$ChangeOutput"
        if ($ChangeExitCode -ne 0) { throw "Nastavení standardního portu $($Change.Device) selhalo (kód $ChangeExitCode): $ChangeOutput" }
    }

    $List = & $Setupc list 2>&1 | Out-String
    Write-Log "Výsledná konfigurace setupc list:`n$List"
    if ($List -notmatch '(?i)RealPortName=COM19' -or $List -notmatch '(?i)RealPortName=COM20') {
        try {
            $Devices = Get-CimInstance -ClassName Win32_PnPEntity -ErrorAction Stop |
                Where-Object { $_.PNPDeviceID -match '(?i)ROOT\\COM0COM' -or $_.Name -match '(?i)com0com' } |
                Select-Object Name, PNPClass, PNPDeviceID, ConfigManagerErrorCode
            Write-Log "Stav zařízení com0com ve Windows: $($Devices | ConvertTo-Json -Compress -Depth 3)"
        } catch {
            Write-Log "Stav zařízení com0com se nepodařilo přes WMI zjistit: $($_.Exception.Message)"
        }
        throw "setupc nepotvrdil standardní porty COM19 a COM20:`n$List"
    }

    # Verify through .NET's Windows COM-port enumeration, which external apps
    # commonly use. setupc list alone only confirms com0com's private config.
    $VisiblePorts = @()
    for ($Attempt = 0; $Attempt -lt 15; $Attempt++) {
        $VisiblePorts = [System.IO.Ports.SerialPort]::GetPortNames()
        if (($VisiblePorts -contains 'COM19') -and ($VisiblePorts -contains 'COM20')) { break }
        Start-Sleep -Seconds 1
    }
    Write-Log "Windows/.NET enumerace COM portů: $($VisiblePorts -join ', ')"
    if (($VisiblePorts -notcontains 'COM19') -or ($VisiblePorts -notcontains 'COM20')) {
        throw 'setupc pár nastavil, ale Windows jej zatím nevystavuje jako běžné sériové porty COM19 a COM20.'
    }
    Write-Log 'Standardní pár COM19 ↔ COM20 je nakonfigurován a viditelný pro aplikace.'
    }
    finally {
        Pop-Location
    }
    exit 0
}
catch {
    Write-Log "CHYBA: $($_.Exception.Message)"
    exit 1
}
