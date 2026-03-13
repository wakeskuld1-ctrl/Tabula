@echo off
set regpath="HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\ODBC Drivers"
REG QUERY %regpath% /s /reg:64|find "YashanDB"
IF ERRORLEVEL 1 (
 REG ADD "HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\ODBC Drivers" /v "YashanDB" /d Installed /reg:64
) ELSE (
 echo YashanDB ODBC driver entry already exists. Proceeding to update keys.
)

REG ADD "HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\YashanDB" /v APILevel /d 1 /f /reg:64
REG ADD "HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\YashanDB" /v ConnectFunctions /d YYN /f /reg:64
REG ADD "HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\YashanDB" /v Driver /d "%~dp0yas_odbc.dll" /f /reg:64
REG ADD "HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\YashanDB" /v DriverODBCVer /d 03.50 /f /reg:64
REG ADD "HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\YashanDB" /v FileUsage /d 0 /f /reg:64
REG ADD "HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\YashanDB" /v Setup /d "%~dp0yas_odbc.dll" /f /reg:64
REG ADD "HKEY_LOCAL_MACHINE\SOFTWARE\ODBC\ODBCINST.INI\YashanDB" /v SQLLevel /d 1 /f /reg:64
