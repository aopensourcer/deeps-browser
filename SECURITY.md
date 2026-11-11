# Sicurezza

Deeps è un browser: tratta sessioni, cookie, cronologia e credenziali. Le segnalazioni di
sicurezza sono importanti.

## Segnalare una vulnerabilità

**Non aprire una issue pubblica.** Usa i
[GitHub Security Advisories](https://github.com/aopensourcer/deeps-browser/security/advisories/new):
la segnalazione resta privata tra te e l'autore.

Se non puoi usare GitHub, contatta l'autore (`@aopensourcer`) tramite un recapito privato
indicato nel profilo.

Includi, se possibile:

- versione di Deeps e di Windows/WebView2;
- passi per riprodurre il problema;
- impatto: cosa può fare un attaccante, con quali privilegi;
- eventuale proof of concept, senza dati reali di terzi.

## Cosa aspettarsi

- Conferma di ricezione appena possibile (best effort: il progetto è portato avanti da una
  persona).
- Aggiornamenti sull'avanzamento e, se richiesto, credito nella nota di rilascio.
- Divulgazione coordinata: chiediamo di non pubblicare il dettaglio prima della correzione.

## Ambito

Rientrano nel perimetro: la shell Rust, il bridge IPC, il server di loopback delle pagine
interne, la cassaforte delle password, la gestione dei download e l'incorporamento dei
contenuti web.

Non sono vulnerabilità di Deeps i problemi del motore WebView2 o di Windows: vanno
segnalati a Microsoft. Se però una configurazione di Deeps li rende sfruttabili, segnalalo
comunque.
