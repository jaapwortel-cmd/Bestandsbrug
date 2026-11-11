# Bestandsbrug / Filebridge

**Nederlands** · [English below](#english)

Bestandsbrug is een kleine Windows-app van Jaap Wortel. Je kunt er twee dingen mee doen:

1. de apps van Jaap downloaden uit een lijst;
2. zelf een bestand naar Jaap sturen (uploaden).

Alles gaat via pCloud. Je hoeft je niet aan te melden. De app werkt goed met een schermlezer (JAWS, NVDA, Verteller)
en helemaal met het toetsenbord. Er is ook een versie voor de browser: één los bestand, `Bestandsbrug.html`.

De app is er in het Nederlands (Bestandsbrug) en in het Engels (Filebridge).

## Bouwen

Je hebt Windows nodig. Open PowerShell in deze map en typ:

```
powershell -ExecutionPolicy Bypass -File .\bouw-alles.ps1
```

Het script installeert zelf wat nodig is (Rust en Inno Setup, gratis, met winget). Daarna staat alles in de map
`oplevering`: het installatiebestand, het HTML-bestand, `versie.txt` en de privacyverklaring.

Tests: `cargo test`.

## Privacy

Lees de privacyverklaring in [talen/privacy-nl.txt](talen/privacy-nl.txt). In de app staat hij onder Help.

---

## English

Filebridge is a small Windows app by Jaap Wortel. You can do two things with it:

1. download Jaap's apps from a list;
2. send a file to Jaap yourself (upload).

Everything goes through pCloud. You do not need to sign in. The app works well with a screen reader
(JAWS, NVDA, Narrator) and fully with the keyboard. There is also a browser version: one single file, `Bestandsbrug.html`.

The app is in Dutch (Bestandsbrug) and in English (Filebridge).

### Build

You need Windows. Open PowerShell in this folder and type:

```
powershell -ExecutionPolicy Bypass -File .\bouw-alles.ps1
```

The script installs what it needs by itself (Rust and Inno Setup, free, with winget). After that, everything is
in the folder `oplevering`: the installer, the HTML file, `versie.txt` and the privacy statement.

Tests: `cargo test`.

### Privacy

Read the privacy statement in [talen/privacy-en.txt](talen/privacy-en.txt). In the app, it is under Help.
