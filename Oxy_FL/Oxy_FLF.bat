@echo off
setlocal

set output=C:\Users\peter\kDrive\Documents\Listes\multimedia.txt

if exist "%output%" del "%output%"

for %%d in (
    "E:\Animes"
    "G:\Series"
    "H:\Reste\Ebooks"
) do (
    dir %%d /ad /b /s 2>nul >> "%output%"
)

echo Terminé.
