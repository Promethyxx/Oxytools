@echo off
setlocal enabledelayedexpansion

:: Liste des emplacements à traiter (en s'assurant que les chemins sont entre guillemets pour éviter des problèmes avec les espaces)
set emplacements="E:\Animes" "H:\Docs" "H:\Docs Nat" "F:\Films" "E:\Films Animes" "G:\Series" "H:\Shows" "H:\Reste"

:: Chemin vers kDrive
set kDrive="C:\Users\peter\kDrive\Documents\Listes"

:: Boucle à travers chaque emplacement
for %%d in (%emplacements%) do (
    echo.
    echo Traitement du dossier: %%d
    :: Vérifier si le dossier existe
    if exist %%d (
        :: Lister tous les fichiers dans le dossier et ses sous-dossiers
        dir %%d /b /a-d /s > "%kDrive%\%%~nd.txt"
        :: Vérifier si le fichier créé est vide
        for %%f in ("%kDrive%\%%~nd.txt") do (
            if %%~zf==0 (
                del "%kDrive%\%%~nd.txt"
            )
        )
    ) else (
        echo Le dossier %%d n'existe pas ou est inaccessible.
    )
)

:: Message de fin
echo Tous les fichiers ont été listés et enregistrés dans kDrive.