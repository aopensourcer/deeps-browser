# Contribuire a Deeps

Grazie per l'interesse. Bug report, idee e pull request sono benvenuti.

## Prima di iniziare

- Cerca tra le [issue](https://github.com/aopensourcer/deeps-browser/issues) esistenti: forse
  il problema è già segnalato.
- Per modifiche grandi (nuove funzioni, refactor, cambi di architettura) apri prima una issue
  e concordiamo l'approccio. Evita PR enormi senza discussione.
- Per i problemi di sicurezza **non** aprire una issue pubblica: leggi [SECURITY.md](SECURITY.md).

## Setup

Requisiti: Windows 10/11, Rust stable (toolchain MSVC), Node 20.9+, WebView2 Runtime.

```powershell
npm --prefix chrome install
npm --prefix chrome run build   # genera chrome/out
cargo run                       # avvia l'app
```

Sviluppo della UI con hot reload, dentro l'app vera:

```powershell
npm --prefix chrome run dev
$env:DEEPS_DEV_URL = "http://localhost:3000"
cargo run
```

Senza Rust, `npm --prefix chrome run dev` apre la UI su `http://localhost:3000` con un
bridge finto (`chrome/lib/ipc.ts`).

## Prima di aprire una PR

```powershell
npm --prefix chrome run typecheck
npm --prefix chrome run build
cargo test
cargo build --release
```

- Mantieni lo stile del codice circostante: Rust semplice e senza dipendenze inutili,
  TypeScript senza librerie di stato esterne, CSS nel foglio esistente.
- Non introdurre telemetria, analytics o chiamate di rete non dichiarate: la privacy è
  parte del prodotto.
- Se aggiungi stringhe visibili, aggiungi la chiave sia in italiano sia in inglese
  (`chrome/lib/i18n.ts`).
- Aggiorna il README se cambia il comportamento visibile all'utente.
- Una PR = una cosa. Descrivi il problema, la soluzione e come l'hai provata.

## Licenza dei contributi

Deeps è distribuito con doppia licenza: **AGPL-3.0** per il pubblico e una licenza
commerciale per chi non può rispettare l'AGPL (vedi [LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md)).

Inviando una pull request dichiari che:

1. il contributo è tuo o hai il diritto di inviarlo;
2. lo rilasci sotto AGPL-3.0;
3. concedi all'autore di Deeps il diritto di distribuirlo anche con una licenza
   commerciale, senza compensi aggiuntivi.

Se il contributo contiene codice di terzi, indica la provenienza e la licenza nella PR.
