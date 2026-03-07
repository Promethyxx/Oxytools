Set WshShell = CreateObject("WScript.Shell")

' Lancer le premier script en arrière-plan et attendre qu'il termine
WshShell.Run """C:\Users\peter\Documents\GitHub\Oxytools\Oxy_FL\Oxy_FLF.bat""", 0, True

' Lancer le deuxième script en arrière-plan et attendre qu'il termine
WshShell.Run """C:\Users\peter\Documents\GitHub\Oxytools\Oxy_FL\Oxy_FL.bat""", 0, True

' Terminer le VBS proprement pour que le Planificateur affiche 0x0
WScript.Quit