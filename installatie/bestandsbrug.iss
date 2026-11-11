; Installatieprogramma van Bestandsbrug (Engels: Filebridge), gemaakt met Inno Setup 6 (gratis).
; Bouwen: bouw-alles.ps1 (die geeft /DVersie=... mee uit src\instellingen.rs en zet de bestanden klaar
; in de map installatiemap). Installeert alleen voor de gebruiker zelf, zonder beheerdersrechten, in
; %LOCALAPPDATA%\Programs\Bestandsbrug. De instellingen staan in %APPDATA%\Bestandsbrug en blijven bewaard
; bij een update (taal, weergave en onderbroken downloads).
; De privacyverklaring komt op een eigen pagina, in de taal van het installatieprogramma (alleen informatie).
; Bestandsbrug gebruikt geen WebView2 of andere extra onderdelen; die hoeven dus niet geïnstalleerd te worden.

#ifndef Versie
  #define Versie "0.0.0"
#endif

[Setup]
AppId={{6F1C2B7E-4D8A-4E39-9B1F-2A7C5D3E8B41}
AppName={cm:AppNaam}
AppVersion={#Versie}
AppVerName={cm:AppNaam} {#Versie}
AppPublisher=Jaap Wortel
VersionInfoVersion={#Versie}
VersionInfoProductName=Bestandsbrug
DefaultDirName={localappdata}\Programs\Bestandsbrug
DefaultGroupName={cm:AppNaam}
DisableProgramGroupPage=yes
DisableWelcomePage=yes
DisableDirPage=yes
DisableReadyPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\oplevering
OutputBaseFilename=Bestandsbrug-Setup
SetupIconFile=..\assets\bestandsbrug.ico
UninstallDisplayIcon={app}\Bestandsbrug.exe
UninstallDisplayName={cm:AppNaam}
ShowLanguageDialog=yes
LanguageDetectionMethod=uilanguage
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no

[Languages]
; Nederlands staat voorop. De privacyverklaring komt uit talen\privacy-nl.txt en talen\privacy-en.txt
; (bouw-alles.ps1 zet een kopie met BOM in installatiemap, zodat de letters goed zijn).
Name: "nl"; MessagesFile: "compiler:Languages\Dutch.isl"; InfoBeforeFile: "..\installatiemap\Privacyverklaring.txt"
Name: "en"; MessagesFile: "compiler:Default.isl"; InfoBeforeFile: "..\installatiemap\Privacy-statement.txt"

[CustomMessages]
nl.AppNaam=Bestandsbrug
en.AppNaam=Filebridge
nl.Bureaublad=Een snelkoppeling op het bureaublad zetten
en.Bureaublad=Put a shortcut on the desktop
nl.NuStarten=Bestandsbrug nu starten
en.NuStarten=Start Filebridge now
nl.PrivacyKop=Privacyverklaring
en.PrivacyKop=Privacy statement
nl.PrivacyUitleg=Lees de privacyverklaring met de pijltjestoetsen. Je hoeft niets te accepteren. Ga verder met Enter of de Spatie.
en.PrivacyUitleg=Read the privacy statement with the arrow keys. You do not need to accept anything. Continue with Enter or Space.

[Tasks]
; Zonder kopje erboven: dan leest de schermlezer het vakje gewoon voor.
Name: "bureaublad"; Description: "{cm:Bureaublad}"

[InstallDelete]
Type: files; Name: "{app}\Bestandsbrug.exe"
Type: files; Name: "{app}\*.txt"

[Files]
Source: "..\installatiemap\Bestandsbrug.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\installatiemap\Privacyverklaring.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\installatiemap\Privacy-statement.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\installatiemap\LEESMIJ.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\installatiemap\Wijzigingen.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; De snelkoppeling krijgt de naam in de taal van het installatieprogramma.
Name: "{userprograms}\{cm:AppNaam}"; Filename: "{app}\Bestandsbrug.exe"
Name: "{userdesktop}\{cm:AppNaam}"; Filename: "{app}\Bestandsbrug.exe"; Tasks: bureaublad

[Run]
; Gewone installatie: een vakje "nu starten". Een update (stil) start de app vanzelf weer.
Filename: "{app}\Bestandsbrug.exe"; Description: "{cm:NuStarten}"; Flags: nowait postinstall skipifsilent
Filename: "{app}\Bestandsbrug.exe"; Flags: nowait; Check: IsStil

[UninstallDelete]
Type: filesandordirs; Name: "{app}"

[Code]
// Stil installeren (een update vanuit de app): dan start de app daarna vanzelf weer.
function IsStil: Boolean;
begin
  Result := WizardSilent;
end;

// De pagina met de privacyverklaring: een duidelijke kop en uitleg, de focus in de tekst (lezen met de
// pijltjes) en verder met Enter (de standaardknop) of de Spatie.
procedure PrivacyToets(Sender: TObject; var Key: Word; Shift: TShiftState);
begin
  if Key = 32 then
  begin
    Key := 0;
    WizardForm.NextButton.OnClick(WizardForm.NextButton);
  end;
end;

procedure InitializeWizard;
begin
  WizardForm.InfoBeforeMemo.OnKeyDown := @PrivacyToets;
end;

procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpInfoBefore then
  begin
    WizardForm.PageNameLabel.Caption := CustomMessage('PrivacyKop');
    WizardForm.PageDescriptionLabel.Caption := CustomMessage('PrivacyUitleg');
    WizardForm.InfoBeforeClickLabel.Caption := '';
    WizardForm.ActiveControl := WizardForm.InfoBeforeMemo;
  end
  else
    WizardForm.ActiveControl := WizardForm.NextButton;
end;
