Set WshShell = CreateObject("WScript.Shell")

' Lancer le script en arrière-plan et attendre qu'il termine
WshShell.Run """C:\Users\peter\Documents\GitHub\Oxytools\Oxy_Backup\Oxy_Backup.bat""", 0, True

' Terminer le VBS proprement pour que le Planificateur affiche 0x0
WScript.Quit
