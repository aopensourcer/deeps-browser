[English](README.md) · **Italiano**

# Deeps

Un browser desktop minimale: shell nativa in **Rust**, interfaccia in **Next.js**.

Niente Electron e niente framework pesanti sopra il motore: `tao` crea la finestra,
`wry` (WebView2 su Windows) crea i webview. Il chrome del browser e una normale app
Next.js esportata in statico e incorporata nel binario.

## Download

Le build per Windows sono nella pagina
[Releases](https://github.com/aopensourcer/deeps-browser/releases): scarica
`Deeps-<versione>-win64.zip`, scompattala e avvia `Deeps.exe`.

Serve Windows 10/11 con il **WebView2 Runtime** (preinstallato su Windows 11 e con Edge
recente). L'eseguibile non è firmato: al primo avvio SmartScreen può chiedere *Ulteriori
informazioni → Esegui comunque*. L'hash SHA-256 del file è nelle note della release.

**Stato: beta (`0.1.0-beta.1`).** Le funzioni elencate qui sotto funzionano, ma mancano
installer, aggiornamenti automatici e firma del codice: vedi [Limiti noti](#limiti-noti).
Le segnalazioni sono benvenute, leggi [CONTRIBUTING.md](CONTRIBUTING.md).

```
+--------------------------------------------------+
|  webview "chrome" (88px)                         |  tab strip + toolbar + controlli finestra
+--------------------------------------------------+
|  webview della tab attiva (uno per tab)          |  pagina web
+--------------------------------------------------+
```

La finestra e **frameless**: i pulsanti riduci / massimizza / chiudi fanno parte della
UI, e il ridimensionamento dai bordi e gestito via JavaScript.

## Funzioni

- Tab multiple con favicon reali, indicatore di caricamento, chiusura, riordino via drag.
- **Tab verticali** con pulsante a icona: quando attivo la barra si restringe a sole
  favicon appena il puntatore esce, e torna larga dopo un passaggio del mouse di 150 ms;
  apertura e chiusura sono animate a 180 ms.
- Forma delle tab (arrotondata, quadrata, pill), colore d'accento e sfondi/wallpaper,
  inclusa una **foto personale** usata come sfondo.
- **Lingua dell'interfaccia** italiana e inglese, con inglese come predefinito.
- Barra indirizzi intelligente: URL, host, `localhost`, path, ricerca.
- Ricerca con **8 motori** e sintassi `!iniziale` (es. `!g rust webview`).
- **25 bang di sito** (`!gh`, `!yt`, `!am`, …) che hanno precedenza sui bang a lettera.
- **Traduttore integrato**: popup dal vivo sulla pagina corrente, con lingua di partenza
  e di arrivo, traduzione dei testi visibili e dei contenuti aggiunti dopo, e ripristino.
- **VPN gratuita** tramite Cloudflare WARP, con stato reale e istruzioni se manca.
- Pagina **Cronologia** con ricerca, eliminazione singola e svuotamento.
- **Preferiti** con stella nella toolbar e pagina dedicata.
- **Download** con pannello rapido dalla toolbar: mini-elenco con avanzamento (anello
  bianco sull'icona), eliminazione e scorciatoia a `deeps://downloads`.
- **Cartella scelta a ogni download** in una finestra Deeps (il dialogo di salvataggio
  nativo di WebView2 non compare), con opzione "Salva sempre qui" e cartella predefinita
  configurabile. I nomi restano univoci e le notifiche segnalano inizio e fine.
- **Password manager** locale con cassaforte cifrata.
- Salvataggio login **su conferma**: nessuna credenziale viene salvata senza un clic.
- Stato **passkey** basato sull'autenticatore di Windows/WebView2.
- **Impostazioni** (lingua, motore, scorciatoie, adblock, tema, sfondo con foto, cartella
  download, traduttore) con effetto immediato.
- **Importa preferiti** da Chrome, Edge, Brave, Vivaldi, Chromium, Opera e Opera GX.
- **Browser predefinito**: registra Deeps in Windows e apre la pagina delle app predefinite.
- **Adblock** integrato a livello DNS, attivabile dalle impostazioni.
- Menu **estensioni** con stato adblock.
- Menu contestuale proprio, con azioni su link, immagini, selezione e pagina.
- Trascinamento di immagini e file nella pagina con anteprima.
- Onboarding iniziale al primo avvio, con scelta della lingua, saltato se esiste già un profilo.
- Scorciatoie attivabili/disattivabili, con indicatore a pallino.
- Tema chiaro/scuro automatico, animazioni contenute, logo vortex.
- Sessione, dimensioni e posizione della finestra persistite.

## Com'e fatto

| File | Ruolo |
| --- | --- |
| `src/main.rs` | event loop, bootstrap, flag WebView2 |
| `src/app.rs` | stato: tab, attivazione, navigazione, layout, overlay, IPC, favicon, controlli finestra |
| `src/assets.rs` | serve l'export di Next nel webview chrome tramite protocollo custom `app://` |
| `src/internal.rs` | server HTTP di loopback: pagine interne, API JSON, redirect di ricerca |
| `src/data.rs` | impostazioni, cronologia, preferiti, download, cassaforte cifrata |
| `src/engines.rs` | tabella motori di ricerca e sintassi `!iniziale` |
| `src/importer.rs` | lettura dei preferiti dai profili Chromium installati |
| `src/defaults.rs` | registrazione in Windows come browser predefinito |
| `src/downloads.rs` | destinazioni dei download, nomi univoci, scelta cartella, apertura file e cartella |
| `src/urlbar.rs` | normalizza l'input: URL, host, localhost, path, ricerca, `deeps://` |
| `src/ipc.rs` | bridge delle pagine: token per tab, scorciatoie, resize, menu contestuale, traduttore, drag |
| `src/adblock.rs` | blocklist host e argomenti WebView2 |
| `src/logo.rs` | logo vortex SVG e icona nativa procedurale |
| `src/store.rs` | sessione, dimensioni e posizione in `session.json` |
| `src/startpage.rs` | helper della new tab page |
| `internal/` | HTML/CSS/JS delle pagine interne (inclusi traduttore e VPN) |
| `chrome/` | UI Next.js 16 (App Router, static export) |

Scelte che vale la pena conoscere:

- **Due webview nella stessa finestra nativa.** Il chrome non e un overlay della pagina:
  e un webview separato, quindi il sito non puo toccarlo ne leggerlo.
- **Le pagine non ricevono API privilegiate.** L'unico canale e `ipc` con un token casuale
  per tab, iniettato prima degli script della pagina: puo solo inoltrare azioni in whitelist.
- **Pagine interne su loopback.** Una vera origine (`http://127.0.0.1:<porta effimera>`)
  evita i problemi di `about:blank` e permette API con controllo di token e Origin.
- **Stato in push.** Rust manda l'intero stato al chrome con `evaluate_script`; i comandi
  tornano indietro come richieste con `id` e risposta.
- **Tab lazy.** Il webview di una tab viene creato solo alla prima attivazione.
- **`CHROME_HEIGHT` (88) e duplicato in `src/app.rs` e nel CSS.** Se cambi l'altezza della
  toolbar, aggiorna entrambi.

## Pagine interne

| Indirizzo | Pagina |
| --- | --- |
| `deeps://start` | nuova tab |
| `deeps://history` | cronologia |
| `deeps://bookmarks` | preferiti |
| `deeps://downloads` | download |
| `deeps://passwords` | password |
| `deeps://translate` | traduttore a pagina intera |
| `deeps://vpn` | VPN gratuita (Cloudflare WARP) |
| `deeps://settings` | impostazioni |

L'indirizzo mostrato nella barra e `deeps://...`, ma il contenuto vive sul server di
loopback locale. Le pagine interne non sono salvabili nei preferiti.

## Ricerca e `!iniziale`

| Iniziale | Motore |
| --- | --- |
| `!d` | DuckDuckGo |
| `!g` | Google |
| `!b` | Bing |
| `!s` | Startpage |
| `!y` | Yahoo |
| `!e` | Ecosia |
| `!q` | Qwant |
| `!w` | Wikipedia |

`!g termine` cerca su Google, `!w` da solo apre la home di Wikipedia. Se l'iniziale non
esiste, il testo completo viene cercato con il motore predefinito e compare il messaggio
"non supportiamo questo motore".

### Bang di sito

I bang di sito sono più lunghi di una lettera e vengono controllati **prima** dei motori,
quindi `!gh rust` non viene interpretato come `!g` seguito da `h rust`.

| Bang | Sito | Bang | Sito | Bang | Sito |
| --- | --- | --- | --- | --- | --- |
| `!gh` | GitHub | `!yt` | YouTube | `!rd` | Reddit |
| `!tw` | X | `!ig` | Instagram | `!so` | Stack Overflow |
| `!md` | MDN | `!npm` | npm | `!cr` | crates.io |
| `!am` | Amazon | `!wb` | Wikipedia IT | `!nyt` | New York Times |
| `!bbc` | BBC | `!cnn` | CNN | `!espn` | ESPN |
| `!imdb` | IMDb | `!sp` | Spotify | `!nf` | Netflix |
| `!tg` | Telegram | `!li` | LinkedIn | `!fb` | Facebook |
| `!pin` | Pinterest | `!maps` | Google Maps | `!drive` | Google Drive |
| `!gpt` | ChatGPT | | | | |

Con il solo bang si apre la home del sito; con un testo dopo il bang si apre la ricerca
interna del sito (dove disponibile).

## Scorciatoie

Attivabili dalle impostazioni (e dal pie di pagina del menu). Con le scorciatoie disattivate
restano solo `Esc` per chiudere il menu e i comandi espliciti della UI.

| Tasto | Azione |
| --- | --- |
| `Ctrl+T` | nuova tab |
| `Ctrl+W` | chiudi tab |
| `Ctrl+L` | barra indirizzi |
| `Ctrl+R` | ricarica |
| `Ctrl+K` | menu / estensioni |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | tab successiva / precedente |
| `Ctrl+Shift+I` | DevTools della pagina |
| `Alt+Left` / `Alt+Right` | indietro / avanti |

## Tab verticali e aspetto

Dalle impostazioni si sceglie il layout **orizzontale** o **verticale**. In verticale la
barra laterale occupa 220px e il pulsante a icona accanto al logo abilita la chiusura
automatica: quando il puntatore esce dall'area delle tab, la barra si restringe a 56px e
restano solo le favicon; rientrando con il mouse torna a 220px dopo 150 ms, così un
passaggio veloce non la fa aprire. La transizione dura 180 ms ed è animata sia nel chrome
(CSS) sia nel bordo nativo della pagina, che viene ridimensionato da Rust.

Lo stesso pannello permette di cambiare forma delle tab (arrotondata, quadrata, pill),
colore d'accento e wallpaper. La chiusura automatica è solo uno stato di sessione: non
viene salvata nel profilo.

Tra gli sfondi c'è **Foto personale**: si sceglie un'immagine dalle impostazioni, viene
ridimensionata nel browser (max 1920px, JPEG) e salvata come data URL nel profilo. Chrome
e pagine interne la mostrano dietro un velo scuro, così i testi restano leggibili; il
chrome la carica dalla rotta interna `/wallpaper` invece di trasportarla nello stato.

## Download

L'icona download della toolbar apre sempre un **pannello rapido** sotto la barra (non la pagina
intera), sia con le tab in alto sia con la sidebar verticale: mostra gli ultimi file, la
percentuale delle operazioni in corso, un pulsante per eliminare una voce e in fondo
**Mostra tutti i download**, che apre `deeps://downloads`.

Il salvataggio non usa il dialogo nativo di WebView2: Deeps intercetta `DownloadStarting`,
mostra una finestra con il nome del file, la cartella proposta e i pulsanti Scegli
cartella / Annulla / Salva. Con "Salva sempre qui" la cartella diventa la predefinita e le
richieste successive non compaiono più (si riattivano dalle impostazioni). La cartella
vuota significa `%USERPROFILE%\Downloads`.

Anche `alert`, `confirm` e `prompt` delle pagine non mostrano più la finestra grigia di
WebView2: Deeps disattiva i dialoghi predefiniti, intercetta `ScriptDialogOpening` e li
disegna nella card del chrome (origine, messaggio, campo di testo per i `prompt`). La
pagina resta bloccata finché non si sceglie OK o Annulla; chiudendo la tab il dialogo
pendente viene annullato. I `beforeunload` vengono accettati in automatico, perché Deeps
chiude le tab senza chiedere conferma.

## Lingua

L'interfaccia è disponibile in **inglese** (predefinito) e **italiano**. La scelta si fa
nel primo avvio oppure da `deeps://settings`; cambiarla ricarica le pagine interne per
applicarla subito. Il chrome Next.js usa un dizionario esplicito (`chrome/lib/i18n.ts`),
mentre le pagine interne partono dai testi italiani e vengono tradotte al volo in inglese.
Un profilo creato prima dell'introduzione della lingua usa l'inglese.

## Importa preferiti e browser predefinito

Da `deeps://settings`, dal menu `Ctrl+K` o dall'ultimo passaggio dell'onboarding:

- **Importa** legge i file `Bookmarks` dei profili Chromium installati (Chrome, Edge,
  Brave, Vivaldi, Chromium, Opera e Opera GX). I doppioni vengono saltati e il riepilogo
  indica quanti preferiti sono stati aggiunti. Firefox usa SQLite e non viene letto.
- **Rendi predefinito** scrive le chiavi per-utente in `HKCU` e apre la pagina *App
  predefinite* di Windows. Windows chiede comunque una conferma: nessun programma può
  diventare predefinito in silenzio. Gli URL passati sulla riga di comando vengono aperti
  in una nuova tab.

## Traduttore

La toolbar e il menu (`Ctrl+K`) aprono **Traduci pagina**: un popup dentro la pagina con
la lingua di partenza (default: rileva) e quella di arrivo (default: italiano). Premendo
Traduci:

- i testi visibili vengono raccolti e tradotti in batch, senza toccare form, codice,
  input o aree modificabili;
- le modifiche successive della pagina vengono osservate e tradotte anch'esse;
- **Ripristina** rimette il testo originale.

Le impostazioni conservano le lingue predefinite e `deeps://translate` offre la stessa
funzione a pagina intera. Le richieste passano dal processo Rust (niente CORS/CSP nella
pagina) e usano il servizio pubblico di Google Translate: serve la connessione a
Internet e il testo della pagina viene inviato al servizio. I batch vengono distanziati
per ridurre i limiti di frequenza; se il servizio risponde 429 il popup lo dice invece
di mostrare una traduzione parziale come completa.

## VPN gratuita

`deeps://vpn` e il pannello VPN mostrano lo stato reale di **Cloudflare WARP** e
permettono connessione e disconnessione. Deeps non incapsula una VPN propria: usa il
client ufficiale `warp-cli`. Se WARP non è installato la pagina lo dichiara e indica
dove scaricarlo, senza fingere una connessione attiva.

## Password, login e passkey

Il salvataggio dei login è **opt-in**: quando un form viene inviato, la pagina mostra un
prompt e le credenziali vengono inoltrate solo dopo un clic su Salva. Se la cassaforte non
esiste o è bloccata, l'interfaccia spiega cosa fare invece di salvare in chiaro.

Le passkey restano gestite dall'autenticatore di piattaforma (Windows Hello/WebView2):
Deeps non le esporta e non le duplica, si limita a riportare se l'autenticatore è
disponibile.

## Dati e privacy

Tutto resta sul computer, in `%APPDATA%\Deeps\`:

| File | Contenuto |
| --- | --- |
| `space.json` | impostazioni (incluse cartella download e foto di sfondo), cronologia (max 4000 voci), preferiti, download |
| `vault.bin` | credenziali cifrate |
| `session.json` | tab aperte, dimensioni e posizione finestra |

La cassaforte usa **Argon2id** (m=19456, t=2, p=1) per derivare la chiave e
**XChaCha20-Poly1305** per cifrare. La password non viene mai salvata. Le credenziali
sono disponibili solo a cassaforte sbloccata e l'interfaccia permette solo la copia,
non la visualizzazione in chiaro. Le credenziali raccolte dai form vengono inviate a
Deeps solo dopo la conferma esplicita nel prompt.

Il traduttore invia i testi visibili della pagina a Google Translate: è una chiamata di
rete come quella del sito, ma vale la pena saperlo. La VPN non aggiunge telemetria
propria: usa `warp-cli` e mostra solo lo stato riportato dal client.

L'adblock traduce una blocklist curata in regole `--host-resolver-rules` di WebView2:
il DNS delle richieste pubblicitarie non viene risolto. Per questo il toggle richiede
un riavvio dell'app.

Eventuali dati della vecchia installazione `Deepy` vengono letti come fallback.

## Requisiti

- Windows 10/11 con **WebView2 Runtime** (preinstallato su Windows 11 e con Edge recente)
- **Rust** stable, toolchain MSVC
- **Node** 20.9+ per buildare la UI

## Comandi

```powershell
# UI (una volta, e dopo ogni modifica alla chrome)
npm --prefix chrome install
npm --prefix chrome run build      # genera chrome/out, incorporata al prossimo cargo build

# App
cargo run                          # debug
cargo run --release

# Test
cargo test
```

Sviluppo della UI con hot reload, dentro l'app vera:

```powershell
npm --prefix chrome run dev
$env:DEEPS_DEV_URL = "http://localhost:3000"
cargo run
```

Senza Rust, `npm --prefix chrome run dev` apre la UI su `http://localhost:3000` con un
bridge finto (`chrome/lib/ipc.ts`): tab, barra indirizzi, menu e pagine sono cliccabili
con dati demo.

## Memoria

Il consumo e tenuto basso con tab lazy, `--process-per-site` e
`--renderer-process-limit=4`; le tab chiuse liberano il webview. L'obiettivo e restare
sotto ~500 MB con un uso normale di 5-10 tab.

## Limiti noti

- WebView2 non supporta le WebExtension: il menu estensioni mostra l'adblock integrato e
  lo dichiara esplicitamente.
- L'adblock è a livello di host, non cosmetico: non rimuove gli spazi vuoti lasciati dagli
  annunci.
- Il ridimensionamento dal bordo superiore è gestito dal chrome; i bordi sinistro, destro e
  inferiore anche dalle pagine. In layout verticale il bordo sinistro non ridimensiona
  perché appartiene alla barra laterale.
- Il traduttore richiede Internet e dipende dal servizio pubblico di Google: con molte
  richieste ravvicinate può rispondere 429. In quel caso il popup mostra l'errore e si può
  riprovare.
- La VPN richiede **Cloudflare WARP** installato (`warp-cli` nel PATH o nel percorso
  standard). Senza client la pagina VPN resta onesta e non simula uno stato connesso.
- Le passkey sono delegate a Windows/WebView2; Deeps non può importarle, esportarle o
  sincronizzarle.
- Il salvataggio login non intercetta i flussi OAuth o i form dentro iframe cross-origin.

## Licenza

Deeps è distribuito sotto **GNU Affero General Public License v3.0**: vedi
[LICENSE](LICENSE). Puoi usarlo, modificarlo e ridistribuirlo alle condizioni dell'AGPL,
che richiede di rendere disponibile il sorgente delle versioni distribuite (incluse quelle
modificate).

Se vuoi usare Deeps in un prodotto closed-source o in un browser brandizzato senza gli
obblighi AGPL, serve una **licenza commerciale**: vedi
[LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md).

## Contribuire

Bug, idee e pull request sono benvenuti: leggi [CONTRIBUTING.md](CONTRIBUTING.md) (in inglese).
Per i problemi di sicurezza non aprire una issue pubblica: vedi [SECURITY.md](SECURITY.md) (in inglese).
