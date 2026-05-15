@echo off
REM Deploy script for Topological Computation Daemon (Windows)
REM
REM Usage:
REM   deploy.bat                    Full deploy, 5000 steps
REM   deploy.bat --steps 200        Short run
REM   deploy.bat --check-only       Checks only, no daemon launch

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "SWARM_DIR=%USERPROFILE%\.swarm"

echo ============================================================
echo   Topological Computation Daemon — Deploy (Windows)
echo ============================================================
echo Script dir: %SCRIPT_DIR%

REM Check Python
where python >nul 2>nul
if errorlevel 1 (
    echo [FAIL] Python not found. Install Python 3.10+ and add to PATH.
    exit /b 1
)
for /f "tokens=*" %%i in ('python --version 2^>^&1') do echo [OK] %%i

REM Step 1: Install dependencies
echo.
echo --- Step 1: Install dependencies ---
python -m pip install spacy pymupdf python-dotenv --quiet 2>nul
if errorlevel 1 (
    echo [FAIL] pip install failed
    exit /b 1
)
echo [OK] Required packages installed

REM Optional packages (don't fail)
python -m pip install web3 ipfshttpclient --quiet 2>nul
echo [OK] Optional packages attempted

REM Step 2: Create directories
echo.
echo --- Step 2: Create directories ---
if not exist "%SWARM_DIR%\blocks" mkdir "%SWARM_DIR%\blocks"
if not exist "%SWARM_DIR%\relations" mkdir "%SWARM_DIR%\relations"
if not exist "%SWARM_DIR%\index" mkdir "%SWARM_DIR%\index"
if not exist "%SWARM_DIR%\output" mkdir "%SWARM_DIR%\output"
echo [OK] %SWARM_DIR%\ created

REM Step 3: Write .env
echo.
echo --- Step 3: Write .env ---
set "ENV_FILE=%SCRIPT_DIR%.env"
if not exist "%ENV_FILE%" (
    (
        echo SEMANTIC_SCHOLAR_API=https://api.semanticscholar.org/graph/v1
        echo ARXIV_API=https://export.arxiv.org/api
        echo UNPAYWALL_EMAIL=hanjunyu2003@proton.me
        echo BRAVE_ANSWER_API_KEY=
        echo BRAVE_SEARCH_API_KEY=
        echo IPFS_API=http://localhost:5001
    ) > "%ENV_FILE%"
    echo [OK] .env created
) else (
    echo [OK] .env already exists
)

REM Step 4: Run tests
echo.
echo --- Step 4: Run tests ---
cd /d "%SCRIPT_DIR%"
python -m pytest test_engine.py -v
if errorlevel 1 (
    echo [FAIL] Tests failed
    exit /b 1
)
echo [OK] All tests passed

REM Step 5: Generate seed + Launch daemon via deploy.py
echo.
echo --- Step 5: Launch via deploy.py ---
cd /d "%SCRIPT_DIR%"
python deploy.py %*

endlocal
