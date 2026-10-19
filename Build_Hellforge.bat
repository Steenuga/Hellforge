@echo off
setlocal EnableExtensions
chcp 65001 >nul
cd /d "%~dp0"
set "ROOT=%CD%"
set "LOG=%ROOT%\hellforge-build.log"
set "FAILED=0"
echo ==================================================
echo              HELLFORGE BUILD ASSISTANT
echo ==================================================
echo.
echo Project: %ROOT%
echo Log: %LOG%
echo.
echo Build started %DATE% %TIME% > "%LOG%"
if not exist "package.json" (echo ERROR: package.json missing. Put this BAT in the hellforge folder.& goto :fail)
if not exist "src-tauri\Cargo.toml" (echo ERROR: src-tauri\Cargo.toml missing.& goto :fail)
set "PATH=%USERPROFILE%\.cargo\bin;%ProgramFiles%\nodejs;%APPDATA%\npm;%PATH%"
where winget >nul 2>&1
if errorlevel 1 (set "HAS_WINGET=0") else (set "HAS_WINGET=1")
call :check_tools
if "%NEED_INSTALL%"=="0" goto :build
if "%HAS_WINGET%"=="0" (
 echo ERROR: winget not found. Install App Installer from Microsoft Store, then retry.
 goto :fail
)
echo.
echo Some build tools are missing. winget can install them.
echo This may require admin permission and large downloads.
choice /C YN /M "Install missing tools automatically"
if errorlevel 2 goto :fail
if "%NODE_MISSING%"=="1" (
 echo Installing Node.js LTS...
 winget install --id OpenJS.NodeJS.LTS --exact --source winget --accept-package-agreements --accept-source-agreements >> "%LOG%" 2>&1
 if errorlevel 1 (echo ERROR: Node.js installation failed. See log.& goto :fail)
)
if "%RUST_MISSING%"=="1" (
 echo Installing Rustup...
 winget install --id Rustlang.Rustup --exact --source winget --accept-package-agreements --accept-source-agreements >> "%LOG%" 2>&1
 if errorlevel 1 (echo ERROR: Rustup installation failed. See log.& goto :fail)
)
if "%CPP_MISSING%"=="1" (
 echo Installing Microsoft C++ Build Tools. This can take a while...
 winget install --id Microsoft.VisualStudio.2022.BuildTools --exact --source winget --accept-package-agreements --accept-source-agreements --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" >> "%LOG%" 2>&1
 if errorlevel 1 (echo ERROR: C++ Build Tools installation failed. See log.& goto :fail)
)
set "PATH=%USERPROFILE%\.cargo\bin;%ProgramFiles%\nodejs;%APPDATA%\npm;%PATH%"
if "%RUST_MISSING%"=="1" (
 where rustup >nul 2>&1
 if not errorlevel 1 (
  echo Installing Rust stable MSVC toolchain...
  rustup toolchain install stable-x86_64-pc-windows-msvc >> "%LOG%" 2>&1
  rustup default stable-x86_64-pc-windows-msvc >> "%LOG%" 2>&1
 )
)
echo.
echo Tool installation attempted. Checking tools again...
call :check_tools
if "%NEED_INSTALL%"=="1" (
 echo Some tools are still unavailable in this terminal.
 echo Restart Windows if requested by installers, then run this BAT again.
 goto :fail
)
:build
echo.
echo [1/3] Installing npm dependencies...
call npm install >> "%LOG%" 2>&1
if errorlevel 1 (echo ERROR: npm install failed. See log.& goto :fail)
echo [2/3] Building Hellforge Windows application...
call npm run tauri build >> "%LOG%" 2>&1
if errorlevel 1 (echo ERROR: Tauri build failed. See log.& goto :fail)
echo [3/3] Locating output...
if exist "%ROOT%\src-tauri\target\release\hellforge.exe" (
 echo.
 echo SUCCESS! Hellforge.exe has been built.
 echo Executable: %ROOT%\src-tauri\target\release\hellforge.exe
 start "" explorer.exe "%ROOT%\src-tauri\target\release"
 goto :done
)
if exist "%ROOT%\src-tauri\target\release\bundle" (
 echo Installer output found.
 start "" explorer.exe "%ROOT%\src-tauri\target\release\bundle"
 goto :done
)
echo ERROR: Build finished but output executable was not found.
goto :fail
:check_tools
set "NEED_INSTALL=0"
set "NODE_MISSING=0"
set "RUST_MISSING=0"
set "CPP_MISSING=0"
where node >nul 2>&1
if errorlevel 1 (set "NODE_MISSING=1"& set "NEED_INSTALL=1"& echo MISSING: Node.js) else (echo OK: Node.js)
where npm >nul 2>&1
if errorlevel 1 (set "NODE_MISSING=1"& set "NEED_INSTALL=1"& echo MISSING: npm) else (echo OK: npm)
where cargo >nul 2>&1
if errorlevel 1 (set "RUST_MISSING=1"& set "NEED_INSTALL=1"& echo MISSING: Rust/Cargo) else (echo OK: Cargo)
set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
if exist "%VSWHERE%" (
 "%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath > "%TEMP%\hellforge_vs.txt" 2>nul
 for %%Z in ("%TEMP%\hellforge_vs.txt") do if %%~zZ EQU 0 (set "CPP_MISSING=1"& set "NEED_INSTALL=1"& echo MISSING: C++ Build Tools) else (echo OK: C++ Build Tools)
) else (set "CPP_MISSING=1"& set "NEED_INSTALL=1"& echo MISSING: C++ Build Tools)
exit /b 0
:fail
echo.
echo BUILD NOT COMPLETED. Check hellforge-build.log for details.
echo If you need help, send me the last lines of that log.
echo.
pause
exit /b 1
:done
echo.
echo Finished. The build log is saved in hellforge-build.log.
echo.
pause
exit /b 0
