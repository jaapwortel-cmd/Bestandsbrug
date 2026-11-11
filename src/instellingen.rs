//! Alle vaste instellingen van de app op één plek: technische naam, versie, de twee openbare pCloud-codes,
//! de alleen-lezen updatelink, de meldtijd van 6 seconden en de tijdslimieten.
//! De naam die de gebruiker ziet (Bestandsbrug of Filebridge) staat NIET hier, maar in talen/nl.txt en talen/en.txt.

/// Technische naam: voor de map in AppData, het logbestand, Bestandsbrug-Setup.exe en Bestandsbrug.html.
/// Deze naam is in alle talen hetzelfde, zodat updates en de bouw altijd kloppen.
pub const TECHNISCHE_NAAM: &str = "Bestandsbrug";

/// Het versienummer van de app. Dit is de enige plek. bouw-alles.ps1, het installatiebestand en versie.txt
/// nemen het hier vandaan.
pub const VERSIE: &str = "1.0.0";

/// Code van de openbare pCloud-map "02 Openbaar" (het stuk achter "code=" in de gedeelde link).
pub const PCLOUD_DOWNLOAD_CODE: &str = "kZKkpV7ZSEUm89aPdvkSed1XufCjyVpwGCfk";

/// Code van de pCloud-uploadlink (bestanden komen in de map "Uploads").
pub const PCLOUD_UPLOAD_CODE: &str = "mvG7ZMqpSiwM6Ap7npVdoCl7ULXzQmX6V";

/// De pCloud-servers. De links staan op de Europese server; de andere is de reserve.
pub const PCLOUD_API_SERVERS: [&str; 2] = ["https://eapi.pcloud.com", "https://api.pcloud.com"];

/// Naam die bij een upload aan pCloud wordt doorgegeven (pCloud vraagt een naam van de uploader).
/// Bewust geen persoonlijke gegevens.
pub const PCLOUD_UPLOADER_NAAM: &str = "Bestandsbrug";

/// Optioneel tekstbestand in de openbare map met regels "bestandsnaam = korte uitleg".
pub const UITLEG_BESTAND: &str = "uitleg.txt";

/// De ALLEEN-LEZEN Dropbox-link van de map "Bestandsbrug Updates". Hier komen Bestandsbrug-Setup.exe en versie.txt in.
pub const UPDATE_LINK: &str = "https://www.dropbox.com/scl/fo/d7nrhucshq6vsssrgifrm/AAan3Gqbedxdg8D8CloNatQ?rlkey=k7szkprtcoj1yyppvrllibpoi&st=rhwnhrgk&dl=0";

/// Naam van het installatiebestand en van het versiebestand in de updatemap.
pub const SETUP_BESTAND: &str = "Bestandsbrug-Setup.exe";
pub const VERSIE_BESTAND: &str = "versie.txt";

/// Elke zoveel seconden hoort de schermlezer de voortgang van een download of upload.
pub const MELDING_INTERVAL_SECONDEN: u64 = 6;

/// Hoe vaak de voortgangsbalk op het scherm wordt bijgewerkt (alleen voor de ogen, de schermlezer zegt niets).
pub const BALK_VERVERSEN_MS: u32 = 200;

/// Tijdslimieten in seconden.
pub const LIMIET_VERBINDEN_SECONDEN: u64 = 10;
pub const LIMIET_LIJST_SECONDEN: u64 = 20;
pub const LIMIET_LEZEN_SECONDEN: u64 = 60;
/// De automatische updatecontrole bij het opstarten: kort, zodat hij nooit blijft hangen.
pub const LIMIET_UPDATECONTROLE_SECONDEN: u64 = 15;
/// Hoe lang na het opstarten de automatische updatecontrole begint (in milliseconden).
pub const UPDATECONTROLE_WACHT_MS: u32 = 3000;
/// Grootste updatemap (zip) die we willen downloaden, in bytes.
pub const MAX_UPDATEMAP_BYTES: u64 = 300 * 1024 * 1024;

/// Het logbestand blijft klein: is het groter dan dit, dan blijft alleen het laatste stuk over.
pub const LOG_MAX_BYTES: u64 = 500 * 1024;
pub const LOG_HOUD_BYTES: u64 = 250 * 1024;

/// Grootte van de stukjes waarin bestanden van en naar de schijf gaan (streamen).
pub const STUK_BYTES: usize = 256 * 1024;

/// Achtervoegsel van een half gedownload bestand.
pub const DEEL_ACHTERVOEGSEL: &str = ".bestandsbrug-deel";
