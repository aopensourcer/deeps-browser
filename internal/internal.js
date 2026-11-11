"use strict";

const TOKEN = (document.querySelector('meta[name="deeps-token"]') || {}).content || "";
const LANG = (document.querySelector('meta[name="deeps-lang"]') || {}).content === "it" ? "it" : "en";
const PAGE = document.body.dataset.page || "";

/* Italian → English UI phrases. Longest first so composites win. English is
   the default language; Italian is used when the setting asks for it. */
const PAIRS = [
  ["Impostazioni", "Settings"],
  ["Cronologia", "History"],
  ["Preferiti", "Bookmarks"],
  ["Traduttore", "Translator"],
  ["Nuova tab", "New tab"],
  ["Gestore password", "Password manager"],
  ["Cassaforte password", "Password vault"],
  ["Cassaforte locale cifrata con Argon2id e XChaCha20-Poly1305. Niente cloud.", "Local vault encrypted with Argon2id and XChaCha20-Poly1305. No cloud."],
  ["Verifica del supporto Windows Hello.", "Checks Windows Hello support."],
  ["Poche cose, tutte locali. Le modifiche si applicano subito.", "A few local settings. Changes apply immediately."],
  ["Motore usato dalla barra indirizzi e dalla nuova tab.", "Engine used by the address bar and the new tab."],
  ["Scorciatoia: scrivi ", "Shortcut: type "],
  ["Scorciatoie: ", "Shortcuts: "],
  ["per cercare con un motore specifico. Un motore sconosciuto esegue la ricerca normale e mostra un avviso.", "to search with a specific engine. An unknown engine runs a normal search and shows a notice."],
  ["25 scorciatoie dirette: il testo dopo il bang diventa una ricerca sul sito.", "25 direct shortcuts: the text after the bang becomes a search on that site."],
  ["Esempi: ", "Examples: "],
  [" cerca su GitHub, ", " searches GitHub, "],
  [" su YouTube, ", " on YouTube, "],
  [" apre la home. I bang di due o più lettere hanno la precedenza su quelli dei motori.", " opens the home page. Two-or-more-letter bangs take precedence over engine bangs."],
  ["Disposizione schede", "Tab layout"],
  ["Orizzontale classica oppure verticale con barra laterale.", "Classic horizontal or vertical with a sidebar."],
  ["Orizzontale", "Horizontal"],
  ["Verticale", "Vertical"],
  ["Forma delle schede", "Tab shape"],
  ["Arrotondata, squadrata o a pillola.", "Rounded, square or pill."],
  ["Arrotondate", "Rounded"],
  ["Squadrate", "Square"],
  ["Pillola", "Pill"],
  ["Colore principale", "Accent colour"],
  ["Usato da schede, pulsanti e dettagli.", "Used by tabs, buttons and details."],
  ["Tinta animata per chrome e nuova tab.", "Animated tint for the chrome and new tab."],
  ["Nessuno", "None"],
  ["Griglia", "Grid"],
  ["Stelle", "Stars"],
  ["Salva i login", "Save logins"],
  ["Chiede conferma prima di copiare una password nella cassaforte cifrata.", "Asks before copying a password into the encrypted vault."],
  ["Lingue predefinite del popup “Traduci pagina”. Puoi cambiarle al volo dal popup.", "Default languages of the “Translate page” popup. You can change them on the fly."],
  ["Traduci da", "Translate from"],
  ["Con “Rileva lingua” Deeps riconosce il testo della pagina.", "With “Detect language” Deeps recognises the page text."],
  ["Traduci a", "Translate to"],
  ["Lingua usata dal pulsante traduttore nella barra.", "Language used by the toolbar translator."],
  [" nella barra apre il popup sulla pagina corrente e traduce i testi man mano che compaiono. Il tasto ", " in the toolbar opens the popup on the current page and translates text as it appears. The "],
  [" rimette la pagina originale.", " button restores the original page."],
  ["Scorciatoie da tastiera", "Keyboard shortcuts"],
  [". Il puntino indica che sono attive.", ". The dot shows they are active."],
  ["Adblock integrato", "Built-in Adblock"],
  ["Blocca tracker e annunci a livello di risoluzione DNS. ", "Blocks trackers and ads at DNS level. "],
  ["Richiede il riavvio di Deeps.", "Requires a Deeps restart."],
  ["Dati", "Data"],
  ["Fino a 4000 visite, conservate in locale.", "Up to 4000 visits, stored locally."],
  ["Apri", "Open"],
  ["Svuota", "Clear"],
  ["Usati anche come link rapidi nella nuova tab.", "Also used as quick links on the new tab."],
  ["Cifrata, sbloccabile con la password principale.", "Encrypted, unlocked with the master password."],
  ["Informazioni", "About"],
  ["Shell Rust (tao + wry/WebView2) e interfaccia Next.js. ", "Rust shell (tao + wry/WebView2) and Next.js interface. "],
  ["Estensioni", "Extensions"],
  ["WebView2 non supporta le estensioni Chrome. Adblock è integrato nel browser.", "WebView2 does not support Chrome extensions. Adblock is built into the browser."],
  ["1 integrata", "1 built in"],
  ["Lingua dell’interfaccia. Inglese è il valore predefinito.", "Interface language. English is the default."],
  ["Importa dai browser", "Import from browsers"],
  ["Preferiti da Chrome, Edge, Brave, Vivaldi e Opera.", "Bookmarks from Chrome, Edge, Brave, Vivaldi and Opera."],
  ["Aggiunge i preferiti trovati senza duplicare quelli già presenti.", "Adds the bookmarks found without duplicating existing ones."],
  ["Nessun preferito trovato", "No bookmarks found"],
  [" importati, ", " imported, "],
  [" già presenti", " already present"],
  ["Importazione…", "Importing…"],
  ["Scegli Deeps nella pagina di Windows", "Choose Deeps in the Windows page"],
  ["Importa", "Import"],
  ["Browser predefinito", "Default browser"],
  ["Apre le app predefinite di Windows per confermare.", "Opens Windows Default apps so you can confirm."],
  ["Rendi predefinito", "Make default"],
  ["Ricerca", "Search"],
  ["Aspetto", "Appearance"],
  ["Motore predefinito", "Default engine"],
  ["Bang dei siti", "Site bangs"],
  ["Lingua", "Language"],
  ["Le pagine che vuoi ritrovare. La stella nella barra li aggiunge al volo.", "The pages you want to find again. The toolbar star adds them instantly."],
  ["Aggiungi preferito", "Add bookmark"],
  ["Le pagine visitate, salvate solo su questo computer.", "Visited pages, stored only on this computer."],
  ["Gestore password", "Password manager"],
  ["Blocca", "Lock"],
  ["Passkey", "Passkeys"],
  ["Cassaforte", "Vault"],
  ["Crea la cassaforte", "Create the vault"],
  ["Scegli una password principale. Non viene salvata: se la perdi, le voci non si possono recuperare.", "Choose a master password. It is not saved: if you lose it, entries cannot be recovered."],
  ["Password principale", "Master password"],
  ["Almeno 6 caratteri", "At least 6 characters"],
  ["Ripeti la password", "Repeat the password"],
  ["Crea cassaforte", "Create vault"],
  ["Cifratura…", "Encrypting…"],
  ["Cassaforte creata", "Vault created"],
  ["Cassaforte bloccata", "Vault locked"],
  ["Inserisci la password principale per sbloccare le voci.", "Enter the master password to unlock entries."],
  ["Sblocca", "Unlock"],
  ["Verifica…", "Checking…"],
  ["Nuova voce", "New entry"],
  ["Sito o nome", "Site or name"],
  ["Utente", "Username"],
  ["Nota", "Note"],
  ["Facoltativa", "Optional"],
  ["Salva voce", "Save entry"],
  ["Nessuna voce ancora. Aggiungine una qui sopra.", "No entries yet. Add one above."],
  ["Mostra", "Show"],
  ["Nascondi", "Hide"],
  ["Copia", "Copy"],
  ["Elimina voce", "Delete entry"],
  ["Eliminare ", "Delete "],
  ["Voce eliminata", "Entry deleted"],
  ["Voce salvata", "Entry saved"],
  ["Cassaforte bloccata", "Vault locked"],
  ["Le password non coincidono", "Passwords do not match"],
  ["Password copiata", "Password copied"],
  ["Copia non disponibile", "Copy unavailable"],
  ["Nessun preferito. Aggiungine uno qui o usa la stella nella barra.", "No bookmarks. Add one here or use the star in the toolbar."],
  ["Nessun risultato.", "No results."],
  ["Ancora nessuna pagina visitata.", "No pages visited yet."],
  ["Inserisci un indirizzo", "Enter an address"],
  ["Preferito aggiunto", "Bookmark added"],
  ["Preferito rimosso", "Bookmark removed"],
  ["Rimuovi preferito", "Remove bookmark"],
  ["Rimuovi dalla cronologia", "Remove from history"],
  ["Svuotare tutta la cronologia?", "Clear all history?"],
  ["Cronologia svuotata", "History cleared"],
  ["Oggi", "Today"],
  ["Ieri", "Yesterday"],
  ["Il traduttore di Deeps passa da un proxy locale: nessuna estensione e nessuna chiave API.", "The Deeps translator goes through a local proxy: no extension and no API key."],
  ["integrato", "built in"],
  ["Pulisci", "Clear"],
  ["Traduci", "Translate"],
  ["Traduzione", "Translation"],
  ["Come funziona", "How it works"],
  ["Traduci con Deeps", "Translate with Deeps"],
  ["Scrivi qualcosa da tradurre", "Write something to translate"],
  ["Traduco…", "Translating…"],
  ["rilevato: ", "detected: "],
  ["Traduzione copiata", "Translation copied"],
  ["Il traduttore non ha restituito testo", "The translator returned no text"],
  ["VPN gratuita", "Free VPN"],
  ["Deeps controlla Cloudflare WARP: gratuito, senza account e senza limiti di traffico.", "Deeps checks Cloudflare WARP: free, no account and no traffic limits."],
  ["Aggiorna", "Refresh"],
  ["Verifica.", "Checking."],
  ["Controllo di Cloudflare WARP in corso.", "Checking Cloudflare WARP."],
  ["Connetti", "Connect"],
  ["Disconnetti", "Disconnect"],
  ["Installa Cloudflare WARP", "Install Cloudflare WARP"],
  ["Scarica il client gratuito dal sito ufficiale Cloudflare.", "Download the free client from the official Cloudflare site."],
  ["Completa l'installazione e accetta i termini al primo avvio.", "Complete the installation and accept the terms on first launch."],
  ["Torna in questa pagina: il pulsante Connetti comparirà da solo.", "Come back to this page: the Connect button will appear."],
  ["Deeps non installa nulla da solo e non conosce le tue credenziali Cloudflare.", "Deeps installs nothing by itself and does not know your Cloudflare credentials."],
  ["Apri 1.1.1.1", "Open 1.1.1.1"],
  ["Perché WARP", "Why WARP"],
  ["Comandi usati", "Commands used"],
  ["solo locale", "local only"],
  ["Connesso", "Connected"],
  ["Connessione…", "Connecting…"],
  ["Disconnessione…", "Disconnecting…"],
  [" caratteri", " characters"],
  ["Disconnesso", "Disconnected"],
  ["Da registrare", "Needs registration"],
  ["Disattivato", "Disabled"],
  ["WARP non installato", "WARP not installed"],
  ["Stato sconosciuto", "Unknown status"],
  ["Errore", "Error"],
  ["WARP connesso", "WARP connected"],
  ["WARP disconnesso", "WARP disconnected"],
  ["warp-cli non ha restituito dettagli.", "warp-cli returned no details."],
  ["Passkey non supportate da questo motore.", "Passkeys are not supported by this engine."],
  ["Windows Hello disponibile: le passkey funzionano.", "Windows Hello available: passkeys work."],
  ["Nessun autenticatore di piattaforma disponibile.", "No platform authenticator available."],
  ["Le passkey vengono create e custodite da Windows Hello e da WebView2 durante il login sui siti. Deeps non le vede, non le salva e non le sincronizza.", "Passkeys are created and kept by Windows Hello and WebView2 during site logins. Deeps does not see, save or sync them."],
  ["Impossibile verificare Windows Hello.", "Could not check Windows Hello."],
  ["Salvato", "Saved"],
  ["Riavvia Deeps per applicare Adblock", "Restart Deeps to apply Adblock"],
  ["Rileva lingua", "Detect language"],
  ["Italiano", "Italian"],
  ["Inglese", "English"],
  ["Spagnolo", "Spanish"],
  ["Francese", "French"],
  ["Tedesco", "German"],
  ["Portoghese", "Portuguese"],
  ["Olandese", "Dutch"],
  ["Russo", "Russian"],
  ["Giapponese", "Japanese"],
  ["Cinese (semplificato)", "Chinese (simplified)"],
  ["Coreano", "Korean"],
  ["Arabo", "Arabic"],
  ["Turco", "Turkish"],
  ["Polacco", "Polish"],
  ["Svedese", "Swedish"],
  ["Danese", "Danish"],
  ["Finlandese", "Finnish"],
  ["Norvegese", "Norwegian"],
  ["Ceco", "Czech"],
  ["Greco", "Greek"],
  ["Ebraico", "Hebrew"],
  ["Hindi", "Hindi"],
  ["Indonesiano", "Indonesian"],
  ["Rumeno", "Romanian"],
  ["Ucraino", "Ukrainian"],
  ["Vietnamita", "Vietnamese"],
  ["Thailandese", "Thai"],
  ["I file salvati da Deeps, con il percorso sul disco.", "Files saved by Deeps, with their path on disk."],
  ["Svuota elenco", "Clear list"],
  ["Nessun download. I file che scarichi finiscono qui.", "No downloads. Files you download show up here."],
  ["Svuotare l'elenco dei download? I file restano sul disco.", "Clear the download list? Files stay on disk."],
  ["Elenco download svuotato", "Download list cleared"],
  ["Rimuovi dall'elenco", "Remove from list"],
  ["Download rimosso", "Download removed"],
  ["In corso…", "In progress…"],
  ["Completato", "Completed"],
  ["Non riuscito", "Failed"],
  ["Cartella", "Folder"],
  ["Foto personale", "Personal photo"],
  ["Una tua immagine come sfondo di Deeps, ridimensionata in locale.", "Your own image as the Deeps background, resized locally."],
  ["Scegli foto", "Choose photo"],
  ["Rimuovi", "Remove"],
  ["Cartella dei download", "Downloads folder"],
  ["Dove Deeps salva i file che scarichi.", "Where Deeps saves the files you download."],
  ["Predefinita", "Default"],
  ["Sfoglia…", "Browse…"],
  ["Lascia vuoto per usare la cartella Download di Windows: ", "Leave empty to use the Windows Downloads folder: "],
  ["Chiedi dove salvare", "Ask where to save"],
  ["Mostra una finestra Deeps per scegliere la cartella a ogni download.", "Shows a Deeps window so you can choose the folder for every download."],
  ["Scegli la cartella dei download", "Choose the downloads folder"],
  ["Immagine non valida", "Invalid image"],
  ["Immagine troppo grande", "Image too large"],
  ["Foto aggiornata", "Photo updated"],
  ["Foto rimossa", "Photo removed"],
].sort((a, b) => b[0].length - a[0].length);

function tr(text) {
  let out = String(text);
  if (LANG === "it") return out;
  for (const [it, en] of PAIRS) {
    if (out.includes(it)) out = out.split(it).join(en);
  }
  return out;
}

function translateDom(root) {
  if (!root || LANG === "it") return;
  if (root.nodeType === 3) {
    const value = root.nodeValue;
    const next = tr(value);
    if (next !== value) root.nodeValue = next;
    return;
  }
  if (root.nodeType !== 1 && root.nodeType !== 9) return;
  const tag = root.tagName;
  if (tag === "SCRIPT" || tag === "STYLE") return;
  for (const attribute of ["placeholder", "title", "aria-label"]) {
    if (root.getAttribute && root.hasAttribute && root.hasAttribute(attribute)) {
      const value = root.getAttribute(attribute);
      const next = tr(value);
      if (next !== value) root.setAttribute(attribute, next);
    }
  }
  for (const child of Array.from(root.childNodes)) translateDom(child);
}

translateDom(document);
document.documentElement.lang = LANG;

const ACCENTS = ["#6366f1", "#0ea5e9", "#22c55e", "#f59e0b", "#ef4444", "#ec4899", "#8b5cf6", "#14b8a6"];

async function applyAppearance() {
  try {
    const data = await api("/api/settings");
    const settings = data.settings || {};
    if (settings.accent) document.documentElement.style.setProperty("--accent", settings.accent);
    document.body.dataset.wallpaper = settings.wallpaper || "none";
    if (settings.tabShape) document.body.dataset.shape = settings.tabShape;
    applyPhoto(data.photoUrl, settings.hasPhoto);
  } catch (error) {
    /* appearance is decorative: never block a page on it */
  }
}

function applyPhoto(url, hasPhoto) {
  if (hasPhoto && url) {
    document.body.classList.add("has-photo");
    document.documentElement.style.setProperty("--photo", `url("${url}")`);
  } else {
    document.body.classList.remove("has-photo");
    document.documentElement.style.removeProperty("--photo");
  }
}

applyAppearance();

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

async function api(path, options = {}) {
  const headers = { "X-Deeps-Token": TOKEN };
  if (options.body) headers["Content-Type"] = "application/json";
  const response = await fetch(path, { ...options, headers: { ...headers, ...(options.headers || {}) } });
  const data = await response.json().catch(() => ({}));
  if (!response.ok) throw new Error(data.error || "Errore " + response.status);
  return data;
}

let toastTimer = 0;
function toast(message) {
  const node = document.getElementById("toast");
  if (!node) return;
  node.textContent = tr(message);
  node.classList.add("show");
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => node.classList.remove("show"), 2600);
}

async function copyText(text) {
  try {
    if (navigator.clipboard && window.isSecureContext) {
      await navigator.clipboard.writeText(text);
      return true;
    }
  } catch (error) {}

  try {
    const area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.position = "fixed";
    area.style.top = "-1000px";
    area.style.opacity = "0";
    document.body.appendChild(area);
    area.select();
    const ok = document.execCommand("copy");
    area.remove();
    return ok;
  } catch (error) {
    return false;
  }
}

function hostOf(url) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch (error) {
    return "";
  }
}

function hueOf(seed) {
  let hue = 0;
  for (let i = 0; i < seed.length; i += 1) hue = (hue * 31 + seed.charCodeAt(i)) % 360;
  return hue;
}

function favicon(seed) {
  const label = (seed || "?").trim().charAt(0).toUpperCase() || "?";
  const node = el("span", "fav", label);
  node.style.setProperty("--h", String(hueOf(seed || label)));
  return node;
}

function labelOf(url, title) {
  return title && title.trim() ? title : hostOf(url) || url;
}

const LOCALE = LANG === "it" ? "it-IT" : "en-US";
const timeFormat = new Intl.DateTimeFormat(LOCALE, { hour: "2-digit", minute: "2-digit" });
const dayFormat = new Intl.DateTimeFormat(LOCALE, { day: "numeric", month: "long" });
const fullDayFormat = new Intl.DateTimeFormat(LOCALE, { day: "numeric", month: "long", year: "numeric" });

function sameDay(a, b) {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

function dayLabel(ms) {
  const date = new Date(ms);
  const now = new Date();
  if (sameDay(date, now)) return tr("Oggi");
  const yesterday = new Date(now.getTime() - 86_400_000);
  if (sameDay(date, yesterday)) return tr("Ieri");
  if (date.getFullYear() === now.getFullYear()) return dayFormat.format(date);
  return fullDayFormat.format(date);
}

function deleteButton(title, action) {
  const button = el("button", "iconbtn", "×");
  button.type = "button";
  button.title = title;
  button.addEventListener("click", action);
  return button;
}

function empty(message) {
  return el("div", "empty", message);
}

/** Reads an image file and returns a downscaled JPEG data URL. */
function downscaleImage(file, maxSize, quality) {
  return new Promise((resolve, reject) => {
    const objectUrl = URL.createObjectURL(file);
    const image = new Image();
    image.onload = () => {
      try {
        const limit = maxSize || 1920;
        const scale = Math.min(1, limit / Math.max(image.naturalWidth, image.naturalHeight));
        const width = Math.max(1, Math.round(image.naturalWidth * scale));
        const height = Math.max(1, Math.round(image.naturalHeight * scale));
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const context = canvas.getContext("2d");
        if (!context) throw new Error("Immagine non valida");
        context.drawImage(image, 0, 0, width, height);
        resolve(canvas.toDataURL("image/jpeg", quality || 0.82));
      } catch (error) {
        reject(error);
      } finally {
        URL.revokeObjectURL(objectUrl);
      }
    };
    image.onerror = () => {
      URL.revokeObjectURL(objectUrl);
      reject(new Error("Immagine non valida"));
    };
    image.src = objectUrl;
  });
}

/* ------------------------------------------------------------------ */
/* history                                                             */
/* ------------------------------------------------------------------ */

async function initHistory() {
  const list = document.getElementById("list");
  const search = document.getElementById("search");
  const clear = document.getElementById("clear");
  let timer = 0;

  async function load() {
    try {
      const data = await api(
        "/api/history?limit=800&q=" + encodeURIComponent(search.value.trim()),
      );
      render(data.entries);
    } catch (error) {
      toast(error.message);
    }
  }

  function render(entries) {
    list.textContent = "";
    if (!entries.length) {
      const node = empty(tr(search.value.trim() ? "Nessun risultato." : "Ancora nessuna pagina visitata."));
      list.append(node);
      return;
    }
    let last = "";
    for (const entry of entries) {
      const label = dayLabel(entry.visited);
      if (label !== last) {
        list.append(el("h2", "day", label));
        last = label;
      }
      const row = el("div", "item");
      row.append(favicon(hostOf(entry.url) || labelOf(entry.url, entry.title)));
      const grow = el("div", "grow");
      const link = el("a", "title", labelOf(entry.url, entry.title));
      link.href = entry.url;
      const meta = el("div", "meta", (hostOf(entry.url) || entry.url) + " · " + timeFormat.format(new Date(entry.visited)));
      grow.append(link, meta);
      row.append(grow);
      row.append(
        deleteButton("Rimuovi dalla cronologia", async () => {
          try {
            await api("/api/history/" + entry.id, { method: "DELETE" });
            row.remove();
          } catch (error) {
            toast(error.message);
          }
        }),
      );
      list.append(row);
    }
    translateDom(list);
  }

  search.addEventListener("input", () => {
    clearTimeout(timer);
    timer = setTimeout(load, 140);
  });

  clear.addEventListener("click", async () => {
    if (!confirm("Svuotare tutta la cronologia?")) return;
    try {
      await api("/api/history", { method: "DELETE" });
      toast("Cronologia svuotata");
      load();
    } catch (error) {
      toast(error.message);
    }
  });

  load();
}

/* ------------------------------------------------------------------ */
/* bookmarks                                                           */
/* ------------------------------------------------------------------ */

async function initBookmarks() {
  const list = document.getElementById("list");
  const form = document.getElementById("add");

  async function load() {
    try {
      const data = await api("/api/bookmarks");
      render(data.entries);
    } catch (error) {
      toast(error.message);
    }
  }

  function render(entries) {
    list.textContent = "";
    if (!entries.length) {
      list.append(empty(tr("Nessun preferito. Aggiungine uno qui o usa la stella nella barra.")));
      return;
    }
    for (const entry of entries) {
      const row = el("div", "item");
      row.append(favicon(hostOf(entry.url) || entry.title));
      const grow = el("div", "grow");
      const link = el("a", "title", labelOf(entry.url, entry.title));
      link.href = entry.url;
      grow.append(link, el("div", "meta", hostOf(entry.url) || entry.url));
      row.append(grow);
      row.append(
        deleteButton("Rimuovi preferito", async () => {
          try {
            await api("/api/bookmarks/" + entry.id, { method: "DELETE" });
            row.remove();
            toast("Preferito rimosso");
          } catch (error) {
            toast(error.message);
          }
        }),
      );
      list.append(row);
    }
    translateDom(list);
  }

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const url = form.elements.url.value.trim();
    const title = form.elements.title.value.trim();
    if (!url) {
      toast("Inserisci un indirizzo");
      return;
    }
    try {
      await api("/api/bookmarks", {
        method: "POST",
        body: JSON.stringify({ url, title }),
      });
      form.reset();
      toast("Preferito aggiunto");
      load();
    } catch (error) {
      toast(error.message);
    }
  });

  load();
}

/* ------------------------------------------------------------------ */
/* downloads                                                           */
/* ------------------------------------------------------------------ */

async function initDownloads() {
  const list = document.getElementById("list");
  const clear = document.getElementById("clear");
  let timer = 0;

  async function load() {
    clearTimeout(timer);
    try {
      const data = await api("/api/downloads");
      render(data.entries || []);
      if ((data.entries || []).some((entry) => entry.state === "running")) {
        timer = setTimeout(load, 1500);
      }
    } catch (error) {
      toast(error.message);
    }
  }

  function stateLabel(entry) {
    if (entry.state === "running") {
      if (entry.total > 0) {
        return Math.min(100, Math.round((entry.received / entry.total) * 100)) + "%";
      }
      return "In corso…";
    }
    if (entry.state === "failed") return "Non riuscito";
    return "Completato";
  }

  function sizeLabel(bytes) {
    if (!bytes || bytes < 0) return "";
    if (bytes < 1024) return bytes + " B";
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + " KB";
    if (bytes < 1024 * 1024 * 1024) return (bytes / (1024 * 1024)).toFixed(1) + " MB";
    return (bytes / (1024 * 1024 * 1024)).toFixed(2) + " GB";
  }

  function render(entries) {
    list.textContent = "";
    if (!entries.length) {
      list.append(empty(tr("Nessun download. I file che scarichi finiscono qui.")));
      return;
    }
    for (const entry of entries) {
      const row = el("div", "item");
      row.append(favicon(hostOf(entry.url) || entry.name));
      const grow = el("div", "grow");
      grow.append(el("div", "name", entry.name));
      const when = new Date(entry.finished || entry.started);
      grow.append(el("div", "meta", dayLabel(when.getTime()) + " · " + timeFormat.format(when)));
      grow.append(el("div", "path", entry.path));
      if (entry.state === "running") {
        const percent = entry.total > 0 ? Math.min(100, Math.round((entry.received / entry.total) * 100)) : 0;
        const bar = el("div", "progressbar");
        const fill = el("span");
        fill.style.width = percent + "%";
        bar.append(fill);
        grow.append(bar);
        if (entry.total > 0) {
          grow.append(
            el("div", "meta", sizeLabel(entry.received) + " / " + sizeLabel(entry.total)),
          );
        }
      }
      row.append(grow);

      const state = el("span", "state " + entry.state, stateLabel(entry));
      row.append(state);

      if (entry.state !== "running") {
        const open = el("button", "btn small ghost", "Apri");
        open.type = "button";
        open.addEventListener("click", async () => {
          try {
            await api("/api/downloads/" + entry.id + "/open", { method: "POST" });
          } catch (error) {
            toast(error.message);
          }
        });
        row.append(open);

        const folder = el("button", "btn small ghost", "Cartella");
        folder.type = "button";
        folder.addEventListener("click", async () => {
          try {
            await api("/api/downloads/" + entry.id + "/folder", { method: "POST" });
          } catch (error) {
            toast(error.message);
          }
        });
        row.append(folder);
      }

      row.append(
        deleteButton("Rimuovi dall'elenco", async () => {
          try {
            await api("/api/downloads/" + entry.id, { method: "DELETE" });
            row.remove();
            toast("Download rimosso");
          } catch (error) {
            toast(error.message);
          }
        }),
      );
      list.append(row);
    }
    translateDom(list);
  }

  clear.addEventListener("click", async () => {
    if (!confirm("Svuotare l'elenco dei download? I file restano sul disco.")) return;
    try {
      await api("/api/downloads", { method: "DELETE" });
      toast("Elenco download svuotato");
      load();
    } catch (error) {
      toast(error.message);
    }
  });

  load();
}

/* ------------------------------------------------------------------ */
/* password vault                                                      */
/* ------------------------------------------------------------------ */

async function initPasswords() {
  const vault = document.getElementById("vault");
  const lock = document.getElementById("lock");
  let status = { exists: false, unlocked: false, count: 0 };

  async function refresh() {
    status = await api("/api/vault/status");
    render();
  }

  function field(label, name, type, placeholder, autocomplete) {
    const wrap = el("label", "field", label);
    const input = el("input", "text");
    input.type = type || "text";
    input.name = name;
    if (placeholder) input.placeholder = placeholder;
    if (autocomplete) input.autocomplete = autocomplete;
    wrap.append(input);
    return wrap;
  }

  function render() {
    vault.textContent = "";
    lock.hidden = !status.unlocked;

    if (!status.exists) {
      const card = el("section", "card");
      card.append(el("h2", undefined, "Crea la cassaforte"));
      card.append(
        el(
          "p",
          "sub",
          "Scegli una password principale. Non viene salvata: se la perdi, le voci non si possono recuperare.",
        ),
      );
      const form = el("form");
      form.style.marginTop = "14px";
      form.append(field("Password principale", "password", "password", "Almeno 6 caratteri", "new-password"));
      form.append(field("Ripeti la password", "confirm", "password", "", "new-password"));
      const submit = el("button", "btn primary small", "Crea cassaforte");
      submit.type = "submit";
      form.append(submit);
      form.addEventListener("submit", async (event) => {
        event.preventDefault();
        if (form.elements.password.value !== form.elements.confirm.value) {
          toast("Le password non coincidono");
          return;
        }
        submit.disabled = true;
        submit.textContent = "Cifratura…";
        try {
          await api("/api/vault/create", {
            method: "POST",
            body: JSON.stringify({ password: form.elements.password.value }),
          });
          toast("Cassaforte creata");
          await refresh();
        } catch (error) {
          toast(error.message);
          submit.disabled = false;
          submit.textContent = "Crea cassaforte";
        }
      });
      card.append(form);
      vault.append(card);
      translateDom(vault);
      return;
    }

    if (!status.unlocked) {
      const card = el("section", "card");
      card.style.maxWidth = "440px";
      card.append(el("h2", undefined, "Cassaforte bloccata"));
      card.append(el("p", "sub", "Inserisci la password principale per sbloccare le voci."));
      const form = el("form");
      form.style.marginTop = "14px";
      form.append(field("Password principale", "password", "password", "", "current-password"));
      const submit = el("button", "btn primary small", "Sblocca");
      submit.type = "submit";
      form.append(submit);
      form.addEventListener("submit", async (event) => {
        event.preventDefault();
        submit.disabled = true;
        submit.textContent = "Verifica…";
        try {
          await api("/api/vault/unlock", {
            method: "POST",
            body: JSON.stringify({ password: form.elements.password.value }),
          });
          await refresh();
        } catch (error) {
          toast(error.message);
          submit.disabled = false;
          submit.textContent = "Sblocca";
        }
      });
      card.append(form);
      vault.append(card);
      translateDom(vault);
      return;
    }

    renderUnlocked();
  }

  async function renderUnlocked() {
    vault.textContent = "";
    const form = el("form", "card");
    form.autocomplete = "off";
    form.append(el("h2", undefined, "Nuova voce"));
    const grid = el("div", "grid");
    grid.append(field("Sito o nome", "site", "text", "github.com"));
    grid.append(field("Utente", "username", "text", "nome@esempio.it", "off"));
    grid.append(field("Password", "password", "text", "", "new-password"));
    grid.append(field("Nota", "note", "text", "Facoltativa", "off"));
    form.append(grid);
    const submit = el("button", "btn primary small", "Salva voce");
    submit.type = "submit";
    form.append(submit);
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      try {
        await api("/api/vault/entries", {
          method: "POST",
          body: JSON.stringify({
            site: form.elements.site.value,
            username: form.elements.username.value,
            password: form.elements.password.value,
            note: form.elements.note.value,
          }),
        });
        toast("Voce salvata");
        await refresh();
      } catch (error) {
        toast(error.message);
      }
    });
    vault.append(form);

    let entries = [];
    try {
      const data = await api("/api/vault/entries");
      entries = data.entries;
    } catch (error) {
      toast(error.message);
    }

    if (!entries.length) {
      const node = empty(tr("Nessuna voce ancora. Aggiungine una qui sopra."));
      vault.append(node);
      return;
    }

    const list = el("div", "list");
    for (const entry of entries) {
      const row = el("div", "item");
      row.append(favicon(entry.site));
      const grow = el("div", "grow");
      grow.append(el("div", "title", entry.site));
      const meta = el("div", "meta");
      meta.textContent = entry.username || "—";
      if (entry.note) meta.textContent += " · " + entry.note;
      grow.append(meta);
      row.append(grow);

      const secret = el("span", "pw", "••••••••");
      row.append(secret);

      const reveal = el("button", "btn small ghost", "Mostra");
      reveal.type = "button";
      reveal.addEventListener("click", () => {
        const open = reveal.dataset.open === "1";
        reveal.dataset.open = open ? "0" : "1";
        secret.textContent = open ? "••••••••" : entry.password;
        reveal.textContent = open ? "Mostra" : "Nascondi";
      });
      row.append(reveal);

      const copy = el("button", "btn small ghost", "Copia");
      copy.type = "button";
      copy.addEventListener("click", async () => {
        const ok = await copyText(entry.password);
        toast(ok ? "Password copiata" : "Copia non disponibile");
      });
      row.append(copy);

      row.append(
        deleteButton("Elimina voce", async () => {
          if (!confirm("Eliminare " + entry.site + "?")) return;
          try {
            await api("/api/vault/entries/" + entry.id, { method: "DELETE" });
            toast("Voce eliminata");
            await refresh();
          } catch (error) {
            toast(error.message);
          }
        }),
      );
      list.append(row);
    }
    vault.append(list);
    translateDom(vault);
  }

  lock.addEventListener("click", async () => {
    try {
      await api("/api/vault/lock", { method: "POST" });
      await refresh();
      toast("Cassaforte bloccata");
    } catch (error) {
      toast(error.message);
    }
  });

  try {
    await refresh();
  } catch (error) {
    toast(error.message);
  }
}

/* ------------------------------------------------------------------ */
/* settings                                                            */
/* ------------------------------------------------------------------ */

function toggleSwitch(node, on) {
  if (!node) return;
  node.classList.toggle("on", on);
  node.setAttribute("aria-checked", String(on));
}

async function initSettings() {
  const select = document.getElementById("engine");
  const engines = document.getElementById("engines");
  const sites = document.getElementById("sites");
  const shortcuts = document.getElementById("shortcuts");
  const shortcutsDot = document.getElementById("shortcutsDot");
  const adblock = document.getElementById("adblock");
  const clear = document.getElementById("clearHistory");
  const version = document.getElementById("version");
  const layout = document.getElementById("tabLayout");
  const shape = document.getElementById("tabShape");
  const wallpaper = document.getElementById("wallpaper");
  const accent = document.getElementById("accent");
  const accentSwatches = document.getElementById("accentSwatches");
  const saveLogins = document.getElementById("saveLogins");
  const translateFrom = document.getElementById("translateFrom");
  const translateTo = document.getElementById("translateTo");
  const language = document.getElementById("language");
  const importBookmarks = document.getElementById("importBookmarks");
  const makeDefault = document.getElementById("makeDefault");
  const photoPick = document.getElementById("photoPick");
  const photoFile = document.getElementById("photoFile");
  const photoPreview = document.getElementById("photoPreview");
  const photoRemove = document.getElementById("photoRemove");
  const downloadDir = document.getElementById("downloadDir");
  const downloadDirPick = document.getElementById("downloadDirPick");
  const downloadDirDefault = document.getElementById("downloadDirDefault");
  const askDownloadDir = document.getElementById("askDownloadDir");
  let state;

  function fillTranslator() {
    if (translateFrom) {
      translateFrom.textContent = "";
      for (const [code, label] of LANGS) {
        const option = el("option", undefined, label);
        option.value = code;
        translateFrom.append(option);
      }
    }
    if (translateTo) {
      translateTo.textContent = "";
      for (const [code, label] of LANGS) {
        if (code === "auto") continue;
        const option = el("option", undefined, label);
        option.value = code;
        translateTo.append(option);
      }
    }
  }

  function paintPhoto(has) {
    if (photoPreview) {
      photoPreview.hidden = !has;
      photoPreview.style.backgroundImage = has && state.photoUrl ? `url("${state.photoUrl}")` : "";
    }
    if (photoRemove) photoRemove.hidden = !has;
  }

  function paint() {
    const settings = state.settings;
    if (select) select.value = settings.engine;
    toggleSwitch(shortcuts, settings.shortcuts);
    if (shortcutsDot) shortcutsDot.classList.toggle("on", settings.shortcuts);
    toggleSwitch(adblock, settings.adblock);
    toggleSwitch(saveLogins, settings.saveLogins);
    toggleSwitch(askDownloadDir, settings.askDownloadDir !== false);
    if (layout) layout.value = settings.tabLayout;
    if (shape) shape.value = settings.tabShape;
    if (wallpaper) wallpaper.value = settings.wallpaper;
    if (accent) accent.value = settings.accent;
    if (translateFrom) translateFrom.value = settings.translateFrom || "auto";
    if (translateTo) translateTo.value = settings.translateTo || "it";
    if (language) language.value = settings.language || "en";
    if (downloadDir) downloadDir.value = settings.downloadDir || "";
    if (downloadDirDefault) downloadDirDefault.textContent = state.defaultDownloadDir || "";
    if (accentSwatches) {
      for (const node of accentSwatches.children) {
        node.classList.toggle("on", node.dataset.color === String(settings.accent).toLowerCase());
      }
    }
    paintPhoto(Boolean(settings.hasPhoto && state.photoUrl));
    applyPhoto(state.photoUrl, settings.hasPhoto);
    document.documentElement.style.setProperty("--accent", settings.accent);
    document.body.dataset.wallpaper = settings.wallpaper || "none";
  }

  async function save(patch) {
    try {
      state = await api("/api/settings", {
        method: "PUT",
        body: JSON.stringify(patch),
      });
      paint();
      toast("Salvato");
    } catch (error) {
      toast(error.message);
    }
  }

  try {
    state = await api("/api/settings");
  } catch (error) {
    toast(error.message);
    return;
  }

  if (select) {
    select.textContent = "";
    for (const engine of state.engines || []) {
      const option = el("option", undefined, engine.label);
      option.value = engine.key;
      select.append(option);
    }
  }
  if (engines) {
    engines.textContent =
      tr("Scorciatoie: ") + (state.engines || []).map((engine) => "!" + engine.initial).join(" · ");
  }
  if (sites) {
    sites.textContent = "";
    for (const site of state.sites || []) {
      const chip = el("span", "chip");
      chip.append(el("b", undefined, "!" + site.key), document.createTextNode(" " + site.label));
      chip.title = "!" + site.key + " " + site.label;
      sites.append(chip);
    }
  }
  if (accentSwatches) {
    accentSwatches.textContent = "";
    for (const color of ACCENTS) {
      const swatch = el("button", "swatch");
      swatch.type = "button";
      swatch.dataset.color = color;
      swatch.style.background = color;
      swatch.setAttribute("aria-label", "Colore " + color);
      swatch.addEventListener("click", () => {
        if (accent) accent.value = color;
        save({ accent: color });
      });
      accentSwatches.append(swatch);
    }
  }
  if (version) version.textContent = state.version;
  fillTranslator();
  paint();
  translateDom(document);

  if (select) select.addEventListener("change", () => save({ engine: select.value }));
  if (shortcuts) {
    shortcuts.addEventListener("click", () => save({ shortcuts: !state.settings.shortcuts }));
  }
  if (adblock) {
    adblock.addEventListener("click", async () => {
      await save({ adblock: !state.settings.adblock });
      toast("Riavvia Deeps per applicare Adblock");
    });
  }
  if (saveLogins) {
    saveLogins.addEventListener("click", () => save({ saveLogins: !state.settings.saveLogins }));
  }
  if (layout) layout.addEventListener("change", () => save({ tabLayout: layout.value }));
  if (shape) shape.addEventListener("change", () => save({ tabShape: shape.value }));
  if (wallpaper) wallpaper.addEventListener("change", () => save({ wallpaper: wallpaper.value }));
  if (translateFrom) {
    translateFrom.addEventListener("change", () => save({ translateFrom: translateFrom.value }));
  }
  if (translateTo) {
    translateTo.addEventListener("change", () => save({ translateTo: translateTo.value }));
  }
  if (language) {
    language.addEventListener("change", async () => {
      await save({ language: language.value });
      // Static phrases were replaced in place, so reload to repaint the UI in
      // the newly selected language.
      location.reload();
    });
  }
  if (importBookmarks) {
    importBookmarks.addEventListener("click", async () => {
      importBookmarks.disabled = true;
      const label = importBookmarks.textContent;
      importBookmarks.textContent = tr("Importazione…");
      try {
        const report = await api("/api/import", { method: "POST" });
        if (!report.added && !report.skipped) {
          toast("Nessun preferito trovato");
        } else {
          toast(`${report.added} importati, ${report.skipped} già presenti`);
        }
      } catch (error) {
        toast(error.message);
      } finally {
        importBookmarks.disabled = false;
        importBookmarks.textContent = label;
      }
    });
  }
  if (makeDefault) {
    makeDefault.addEventListener("click", async () => {
      makeDefault.disabled = true;
      try {
        await api("/api/default", { method: "POST" });
        toast("Scegli Deeps nella pagina di Windows");
      } catch (error) {
        toast(error.message);
      } finally {
        makeDefault.disabled = false;
      }
    });
  }
  if (accent) {
    accent.addEventListener("change", () => save({ accent: accent.value }));
  }
  if (clear) {
    clear.addEventListener("click", async () => {
      if (!confirm("Svuotare tutta la cronologia?")) return;
      try {
        await api("/api/history", { method: "DELETE" });
        toast("Cronologia svuotata");
      } catch (error) {
        toast(error.message);
      }
    });
  }
  if (photoPick && photoFile) {
    photoPick.addEventListener("click", () => photoFile.click());
    photoFile.addEventListener("change", async () => {
      const file = photoFile.files && photoFile.files[0];
      photoFile.value = "";
      if (!file) return;
      if (!/^image\//.test(file.type)) {
        toast("Immagine non valida");
        return;
      }
      try {
        const dataUrl = await downscaleImage(file, 1920, 0.82);
        await save({ photo: dataUrl });
        toast("Foto aggiornata");
      } catch (error) {
        toast(error.message || "Immagine non valida");
      }
    });
  }
  if (photoRemove) {
    photoRemove.addEventListener("click", async () => {
      await save({ photo: "" });
      toast("Foto rimossa");
    });
  }
  if (downloadDir) {
    downloadDir.addEventListener("change", () => save({ downloadDir: downloadDir.value.trim() }));
  }
  if (downloadDirPick) {
    downloadDirPick.addEventListener("click", async () => {
      try {
        const picked = await api("/api/settings/folder", { method: "POST" });
        if (picked.path) {
          if (downloadDir) downloadDir.value = picked.path;
          await save({ downloadDir: picked.path });
        }
      } catch (error) {
        toast(error.message);
      }
    });
  }
  if (askDownloadDir) {
    askDownloadDir.addEventListener("click", () =>
      save({ askDownloadDir: state.settings.askDownloadDir === false }),
    );
  }
}

/* ------------------------------------------------------------------ */
/* translator                                                          */
/* ------------------------------------------------------------------ */

const LANGS = [
  ["auto", "Rileva lingua"],
  ["it", "Italiano"],
  ["en", "Inglese"],
  ["es", "Spagnolo"],
  ["fr", "Francese"],
  ["de", "Tedesco"],
  ["pt", "Portoghese"],
  ["nl", "Olandese"],
  ["ru", "Russo"],
  ["ja", "Giapponese"],
  ["zh-CN", "Cinese (semplificato)"],
  ["ko", "Coreano"],
  ["ar", "Arabo"],
  ["tr", "Turco"],
  ["pl", "Polacco"],
  ["sv", "Svedese"],
  ["da", "Danese"],
  ["fi", "Finlandese"],
  ["no", "Norvegese"],
  ["cs", "Ceco"],
  ["el", "Greco"],
  ["he", "Ebraico"],
  ["hi", "Hindi"],
  ["id", "Indonesiano"],
  ["ro", "Rumeno"],
  ["uk", "Ucraino"],
  ["vi", "Vietnamita"],
  ["th", "Thailandese"],
];

function langLabel(code) {
  const match = LANGS.find((item) => item[0].toLowerCase() === String(code).toLowerCase());
  return match ? match[1] : code;
}

async function initTranslate() {
  const from = document.getElementById("from");
  const to = document.getElementById("to");
  const source = document.getElementById("source");
  const output = document.getElementById("output");
  const result = document.getElementById("result");
  const count = document.getElementById("count");
  const detected = document.getElementById("detected");
  const run = document.getElementById("translate");
  const clear = document.getElementById("clear");
  const swap = document.getElementById("swap");
  const copy = document.getElementById("copy");

  for (const [code, label] of LANGS) {
    if (from) {
      const option = el("option", undefined, label);
      option.value = code;
      from.append(option);
    }
    if (to && code !== "auto") {
      const option = el("option", undefined, label);
      option.value = code;
      to.append(option);
    }
  }
  if (from) from.value = "auto";
  if (to) to.value = "it";

  try {
    const prefs = await api("/api/settings");
    if (from && prefs.settings && prefs.settings.translateFrom) from.value = prefs.settings.translateFrom;
    if (to && prefs.settings && prefs.settings.translateTo) to.value = prefs.settings.translateTo;
  } catch (error) {
    /* defaults are enough when the settings endpoint is unavailable */
  }

  const params = new URLSearchParams(location.search);
  const seed = params.get("text");
  if (seed && source) source.value = seed.slice(0, 5000);
  const requested = params.get("to");
  if (requested && to) {
    const known = LANGS.some(([code]) => code === requested);
    if (known && requested !== "auto") to.value = requested;
  }

  function updateCount() {
    if (!count || !source) return;
    count.textContent = source.value.length + tr(" caratteri");
  }
  updateCount();

  async function translate() {
    if (!source || !output || !result) return;
    const text = source.value.trim();
    if (!text) {
      toast("Scrivi qualcosa da tradurre");
      source.focus();
      return;
    }
    if (run) {
      run.disabled = true;
      run.textContent = tr("Traduco…");
    }
    try {
      const data = await api("/api/translate", {
        method: "POST",
        body: JSON.stringify({ text, from: from.value, to: to.value }),
      });
      output.textContent = data.text;
      result.hidden = false;
      if (detected) {
        const label = data.detected ? langLabel(data.detected) : "";
        detected.textContent = label ? tr("rilevato: ") + label : "";
        detected.hidden = !label;
      }
    } catch (error) {
      toast(error.message);
    } finally {
      if (run) {
        run.disabled = false;
        run.textContent = tr("Traduci");
      }
    }
  }

  if (source) {
    source.addEventListener("input", updateCount);
    source.addEventListener("keydown", (event) => {
      if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
        event.preventDefault();
        translate();
      }
    });
  }
  if (run) run.addEventListener("click", translate);
  if (clear) {
    clear.addEventListener("click", () => {
      if (source) source.value = "";
      if (result) result.hidden = true;
      if (detected) detected.hidden = true;
      updateCount();
      source && source.focus();
    });
  }
  if (swap) {
    swap.addEventListener("click", () => {
      const target = to.value;
      if (!target || target === "auto") return;
      if (from.value === "auto") {
        from.value = target;
        to.value = "it" === target ? "en" : "it";
      } else {
        const previous = from.value;
        from.value = target;
        to.value = previous;
      }
      if (source && source.value.trim()) translate();
    });
  }
  if (copy) {
    copy.addEventListener("click", async () => {
      if (!output) return;
      const ok = await copyText(output.textContent || "");
      toast(ok ? "Traduzione copiata" : "Copia non disponibile");
    });
  }

  if (source && source.value.trim()) translate();
}

/* ------------------------------------------------------------------ */
/* vpn                                                                 */
/* ------------------------------------------------------------------ */

async function initVpn() {
  const card = document.getElementById("vpn");
  const stateNode = document.getElementById("vpnState");
  const detailNode = document.getElementById("vpnDetail");
  const connect = document.getElementById("connect");
  const disconnect = document.getElementById("disconnect");
  const refresh = document.getElementById("refresh");
  const install = document.getElementById("installCard");

  const labels = {
    connected: "Connesso",
    connecting: "Connessione…",
    disconnected: "Disconnesso",
    unregistered: "Da registrare",
    disabled: "Disattivato",
    missing: "WARP non installato",
    unknown: "Stato sconosciuto",
  };

  function paint(data) {
    if (card) card.dataset.state = data.state || "unknown";
    if (stateNode) stateNode.textContent = tr(labels[data.state] || labels.unknown);
    if (detailNode) {
      detailNode.textContent = data.detail || (data.installed ? tr("warp-cli non ha restituito dettagli.") : "");
    }
    if (install) install.hidden = data.installed !== false;
    if (connect) connect.disabled = !data.installed || data.state === "connected";
    if (disconnect) {
      disconnect.disabled =
        !data.installed || data.state === "disconnected" || data.state === "missing";
    }
  }

  async function load() {
    try {
      paint(await api("/api/vpn/status"));
    } catch (error) {
      if (detailNode) detailNode.textContent = error.message;
      if (stateNode) stateNode.textContent = tr("Errore");
    }
  }

  async function command(action) {
    const button = action === "connect" ? connect : disconnect;
    const original = button ? button.textContent : "";
    if (button) {
      button.disabled = true;
      button.textContent = action === "connect" ? tr("Connessione…") : tr("Disconnessione…");
    }
    try {
      const data = await api("/api/vpn/" + action, { method: "POST" });
      paint(data);
      toast(action === "connect" ? "WARP connesso" : "WARP disconnesso");
    } catch (error) {
      toast(error.message);
      await load();
    } finally {
      if (button) button.textContent = original;
    }
  }

  if (connect) connect.addEventListener("click", () => command("connect"));
  if (disconnect) disconnect.addEventListener("click", () => command("disconnect"));
  if (refresh) refresh.addEventListener("click", load);
  load();
}

/* ------------------------------------------------------------------ */
/* passkeys                                                            */
/* ------------------------------------------------------------------ */

async function initPasskey() {
  const stateNode = document.getElementById("passkeyState");
  const detailNode = document.getElementById("passkeyDetail");
  if (!stateNode) return;

  if (!window.PublicKeyCredential || !PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable) {
    stateNode.textContent = tr("Passkey non supportate da questo motore.");
    stateNode.dataset.state = "off";
    return;
  }

  try {
    const available = await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
    stateNode.textContent = available
      ? tr("Windows Hello disponibile: le passkey funzionano.")
      : tr("Nessun autenticatore di piattaforma disponibile.");
    stateNode.dataset.state = available ? "on" : "off";
    if (detailNode && available) {
      detailNode.textContent =
        tr("Le passkey vengono create e custodite da Windows Hello e da WebView2 durante il login sui siti. Deeps non le vede, non le salva e non le sincronizza.");
    }
  } catch (error) {
    stateNode.textContent = tr("Impossibile verificare Windows Hello.");
    stateNode.dataset.state = "off";
  }
}

if (PAGE === "history") initHistory();
else if (PAGE === "bookmarks") initBookmarks();
else if (PAGE === "downloads") initDownloads();
else if (PAGE === "passwords") {
  initPasswords();
  initPasskey();
} else if (PAGE === "settings") initSettings();
else if (PAGE === "translate") initTranslate();
else if (PAGE === "vpn") initVpn();
