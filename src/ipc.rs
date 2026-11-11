use serde_json::Value;

#[derive(serde::Deserialize)]
pub struct Request {
    pub id: u64,
    pub cmd: String,
    #[serde(default)]
    pub args: Value,
}

pub enum Message {
    Action(&'static str),
    /// One of: north, south, east, west, northeast, northwest, southeast,
    /// southwest. Sent when the pointer goes down on a window edge.
    Resize(&'static str),
    OpenUrl { url: String, new_tab: bool },
    Search(String),
    Translate(String),
    /// A batch of visible page texts that should be translated in place.
    PageTranslate {
        id: u64,
        from: String,
        to: String,
        texts: Vec<String>,
    },
    /// The user changed the languages inside the in-page translator popup.
    TranslatePrefs { from: String, to: String },
    SaveLogin {
        site: String,
        username: String,
        password: String,
    },
}

/// Random-ish per-tab token. Not cryptographic: it only has to stop a page
/// from guessing the value and forging chrome shortcuts.
pub fn new_token(tab: u64) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
        .hash(&mut hasher);
    tab.hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    let a = hasher.finish();

    let mut hasher = DefaultHasher::new();
    a.hash(&mut hasher);
    (&a as *const u64 as usize).hash(&mut hasher);
    let b = hasher.finish();

    format!("{a:016x}{b:016x}")
}

fn web_url(raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.len() > 4096 || !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    Some(url.to_string())
}

/// Accepts only the exact message shapes our injected script sends.
pub fn parse(body: &str, token: &str) -> Option<Message> {
    let value: Value = serde_json::from_str(body).ok()?;
    let object = value.as_object()?;
    if object.get("__k")?.as_str()? != token {
        return None;
    }

    match object.get("k")?.as_str()? {
        "new_tab" => Some(Message::Action("new_tab")),
        "close_tab" => Some(Message::Action("close_tab")),
        "reload" => Some(Message::Action("reload")),
        "focus_address" => Some(Message::Action("focus_address")),
        "next_tab" => Some(Message::Action("next_tab")),
        "prev_tab" => Some(Message::Action("prev_tab")),
        "back" => Some(Message::Action("back")),
        "forward" => Some(Message::Action("forward")),
        "menu" => Some(Message::Action("menu")),
        "devtools" => Some(Message::Action("devtools")),
        "resize" => {
            if object.len() != 3 {
                return None;
            }
            let direction = match object.get("d")?.as_str()? {
                "north" => "north",
                "south" => "south",
                "east" => "east",
                "west" => "west",
                "northeast" => "northeast",
                "northwest" => "northwest",
                "southeast" => "southeast",
                "southwest" => "southwest",
                _ => return None,
            };
            Some(Message::Resize(direction))
        }
        "open_url" => {
            if object.len() != 4 {
                return None;
            }
            Some(Message::OpenUrl {
                url: web_url(object.get("u")?.as_str()?)?,
                new_tab: object.get("n")?.as_bool()?,
            })
        }
        "search" => {
            if object.len() != 3 {
                return None;
            }
            let text = object.get("t")?.as_str()?.trim();
            if text.is_empty() || text.chars().count() > 500 {
                return None;
            }
            Some(Message::Search(text.to_string()))
        }
        "translate" => {
            if object.len() != 3 {
                return None;
            }
            let text = object.get("t")?.as_str()?.trim();
            if text.is_empty() || text.chars().count() > 2000 {
                return None;
            }
            Some(Message::Translate(text.to_string()))
        }
        "page_translate" => {
            if object.len() != 6 {
                return None;
            }
            let id = object.get("i")?.as_u64()?;
            let from =
                crate::data::normalize_language(object.get("f")?.as_str()?, true).ok()?;
            let to = crate::data::normalize_language(object.get("t")?.as_str()?, false).ok()?;
            let list = object.get("x")?.as_array()?;
            if list.is_empty() || list.len() > 60 {
                return None;
            }
            let mut texts = Vec::with_capacity(list.len());
            let mut total = 0usize;
            for item in list {
                let text = item.as_str()?.trim();
                let length = text.chars().count();
                if length == 0 || length > 2000 {
                    return None;
                }
                total += length;
                texts.push(text.to_string());
            }
            if total > 4000 {
                return None;
            }
            Some(Message::PageTranslate {
                id,
                from,
                to,
                texts,
            })
        }
        "translate_prefs" => {
            if object.len() != 4 {
                return None;
            }
            Some(Message::TranslatePrefs {
                from: crate::data::normalize_language(object.get("f")?.as_str()?, true).ok()?,
                to: crate::data::normalize_language(object.get("t")?.as_str()?, false).ok()?,
            })
        }
        "save_login" => {
            if object.len() != 5 {
                return None;
            }
            let site = object.get("s")?.as_str()?.trim();
            let username = object.get("u")?.as_str()?.trim();
            let password = object.get("p")?.as_str()?;
            if site.is_empty()
                || site.chars().count() > 200
                || username.chars().count() > 200
                || password.is_empty()
                || password.chars().count() > 500
            {
                return None;
            }
            Some(Message::SaveLogin {
                site: site.to_string(),
                username: username.chars().take(200).collect(),
                password: password.chars().take(500).collect(),
            })
        }
        _ => None,
    }
}

/// Injected before any page script. The token lives in this closure, so page
/// JavaScript cannot read it; it can only fire the whitelisted actions.
///
/// It also installs the frameless window resizing, the custom context menu,
/// image drag/drop helpers and the "save this login?" prompt. Keeping all of
/// it in the page avoids cross-webview drag and overlay clipping problems.
pub fn page_script(
    token: &str,
    shortcuts: bool,
    save_logins: bool,
    vertical: bool,
    translate_from: &str,
    translate_to: &str,
) -> String {
    let initial = if shortcuts { "true" } else { "false" };
    let save = if save_logins { "true" } else { "false" };
    let vertical = if vertical { "true" } else { "false" };
    let from_json =
        serde_json::to_string(translate_from).unwrap_or_else(|_| "\"auto\"".to_string());
    let to_json = serde_json::to_string(translate_to).unwrap_or_else(|_| "\"it\"".to_string());

    r##"(function () {
  "use strict";
  try { if (window.top !== window.self) return; } catch (error) { return; }

  var token = "__TOKEN__";
  var internal = location.hostname === "127.0.0.1" || location.hostname === "localhost";
  window.__deepsShortcuts = __INITIAL__;
  window.__deepsSaveLogins = __SAVELOGINS__;
  window.__deepsVertical = __VERTICAL__;
  window.__deepsTranslateFrom = __TRANSLATE_FROM__;
  window.__deepsTranslateTo = __TRANSLATE_TO__;

  function post(payload) {
    payload.__k = token;
    try { window.ipc.postMessage(JSON.stringify(payload)); } catch (error) {}
  }
  function send(action) { post({ k: action }); }
  function sendResize(direction) { post({ k: "resize", d: direction }); }

  /* ---------------------------------------------------------------- */
  /* keyboard shortcuts + window edges                                 */
  /* ---------------------------------------------------------------- */

  window.addEventListener("keydown", function (event) {
    if (window.__deepsShortcuts === false) return;
    var key = (event.key || "").toLowerCase();
    if (event.altKey && !event.ctrlKey && !event.metaKey) {
      if (key === "arrowleft") { event.preventDefault(); send("back"); }
      else if (key === "arrowright") { event.preventDefault(); send("forward"); }
      return;
    }
    if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
    var action = null;
    if (key === "t") action = "new_tab";
    else if (key === "w") action = "close_tab";
    else if (key === "l") action = "focus_address";
    else if (key === "r") action = "reload";
    else if (key === "k") action = "menu";
    else if (key === "i" && event.shiftKey) action = "devtools";
    else if (key === "tab") action = event.shiftKey ? "prev_tab" : "next_tab";
    if (!action) return;
    event.preventDefault();
    send(action);
  }, true);

  var edge = 6;
  var zone = "";
  var pointer = document.createElement("style");
  function attachPointer() {
    var root = document.head || document.documentElement;
    if (!root) {
      document.addEventListener("DOMContentLoaded", attachPointer);
      return;
    }
    if (!pointer.parentNode) {
      try { root.appendChild(pointer); } catch (error) {}
    }
  }
  attachPointer();
  function resizeMarker(on) {
    var root = document.documentElement;
    if (!root) return;
    if (on) root.classList.add("deeps-resize");
    else root.classList.remove("deeps-resize");
  }
  function direction(x, y) {
    var width = window.innerWidth;
    var height = window.innerHeight;
    var left = x < edge && window.__deepsVertical !== true;
    var right = x > width - edge;
    var bottom = y > height - edge;
    if (bottom && left) return "southwest";
    if (bottom && right) return "southeast";
    if (bottom) return "south";
    if (left) return "west";
    if (right) return "east";
    return "";
  }
  function cursor(direction) {
    switch (direction) {
      case "north": case "south": return "ns-resize";
      case "east": case "west": return "ew-resize";
      case "northeast": case "southwest": return "nesw-resize";
      case "northwest": case "southeast": return "nwse-resize";
      default: return "";
    }
  }
  window.addEventListener("pointermove", function (event) {
    if (event.buttons !== 0) return;
    var next = direction(event.clientX, event.clientY);
    if (next === zone) return;
    zone = next;
    if (!next) {
      resizeMarker(false);
      pointer.textContent = "";
      return;
    }
    resizeMarker(true);
    pointer.textContent = "html.deeps-resize, html.deeps-resize * { cursor: " + cursor(next) + " !important; }";
  }, true);
  window.addEventListener("pointerdown", function (event) {
    if (event.button !== 0) return;
    var next = direction(event.clientX, event.clientY);
    if (!next) return;
    event.preventDefault();
    event.stopPropagation();
    sendResize(next);
  }, true);
  window.addEventListener("pointerleave", function () {
    zone = "";
    resizeMarker(false);
    pointer.textContent = "";
  }, true);

  if (internal) return;

  /* ---------------------------------------------------------------- */
  /* isolated UI (shadow DOM)                                          */
  /* ---------------------------------------------------------------- */

  var ui = null;
  function deepsUi() {
    if (ui) return ui;
    var host = document.createElement("div");
    host.id = "__deeps_ui";
    host.style.cssText = "all: initial; position: fixed; left: 0; top: 0; width: 0; height: 0; z-index: 2147483647;";
    var root = host.attachShadow({ mode: "open" });
    root.innerHTML =
      "<style>" +
      "*{box-sizing:border-box;margin:0;padding:0;font-family:-apple-system,'Segoe UI Variable Display','Segoe UI',system-ui,sans-serif}" +
      ".layer{position:fixed;inset:0;pointer-events:none}" +
      ".menu{position:fixed;min-width:224px;max-width:310px;padding:5px;border:1px solid rgba(127,127,140,.26);border-radius:12px;background:rgba(24,24,28,.92);backdrop-filter:blur(18px);box-shadow:0 20px 54px rgba(0,0,0,.42);color:#f3f3f5;font-size:12.5px;transform-origin:top left;animation:menu-in .16s cubic-bezier(.2,.9,.24,1);pointer-events:auto;user-select:none}" +
      "@keyframes menu-in{from{opacity:0;transform:scale(.94) translateY(-4px)}to{opacity:1;transform:none}}" +
      ".menu button{display:flex;align-items:center;gap:10px;width:100%;padding:7px 9px;border:0;border-radius:7px;background:transparent;color:inherit;font:inherit;text-align:left;cursor:default;transition:background .09s ease,transform .09s ease}" +
      ".menu button:hover{background:rgba(255,255,255,.10)}" +
      ".menu button:active{transform:scale(.985)}" +
      ".menu button[disabled]{opacity:.38;pointer-events:none}" +
      ".menu .ico{flex:none;display:grid;place-items:center;width:17px;height:17px;border-radius:5px;background:rgba(255,255,255,.07);font-size:10px;color:#c9c9d4}" +
      ".menu .txt{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}" +
      ".menu .sep{height:1px;margin:5px 8px;background:rgba(255,255,255,.10)}" +
      ".menu .danger{color:#ff8f95}" +
      ".toast{position:fixed;left:50%;bottom:26px;transform:translate(-50%,10px);max-width:calc(100vw - 40px);padding:9px 15px;border-radius:999px;background:rgba(24,24,28,.95);color:#f3f3f5;border:1px solid rgba(127,127,140,.26);box-shadow:0 14px 36px rgba(0,0,0,.38);font-size:12px;opacity:0;pointer-events:none;transition:opacity .18s ease,transform .18s ease}" +
      ".toast.show{opacity:1;transform:translate(-50%,0)}" +
      ".drop{position:fixed;inset:0;display:grid;place-items:center;background:rgba(10,10,14,.36);backdrop-filter:blur(3px);pointer-events:auto;animation:fade-in .14s ease}" +
      ".dropcard{padding:28px 40px;border:2px dashed rgba(255,255,255,.36);border-radius:18px;background:rgba(24,24,28,.86);color:#f3f3f5;font-size:14px;text-align:center}" +
      ".dropcard small{display:block;margin-top:5px;color:#a5a5b2;font-size:11.5px}" +
      "@keyframes fade-in{from{opacity:0}to{opacity:1}}" +
      ".preview{position:fixed;inset:0;display:grid;place-items:center;padding:36px;background:rgba(6,6,9,.72);backdrop-filter:blur(9px);pointer-events:auto;animation:fade-in .16s ease}" +
      ".preview[hidden],.drop[hidden]{display:none}" +
      ".preview img{max-width:min(1100px,92vw);max-height:calc(100vh - 120px);border-radius:12px;box-shadow:0 30px 80px rgba(0,0,0,.5)}" +
      ".pvbar{position:fixed;top:18px;left:50%;transform:translateX(-50%);display:flex;align-items:center;gap:14px;padding:8px 10px 8px 15px;border-radius:999px;background:rgba(24,24,28,.94);color:#e9e9ef;font-size:11.5px}" +
      ".pvbar button{border:0;border-radius:999px;padding:6px 12px;background:rgba(255,255,255,.12);color:#fff;font:inherit;cursor:default}" +
      ".login{position:fixed;right:18px;bottom:18px;width:min(330px,calc(100vw - 36px));padding:15px;border:1px solid rgba(127,127,140,.3);border-radius:15px;background:rgba(24,24,28,.95);backdrop-filter:blur(18px);box-shadow:0 24px 60px rgba(0,0,0,.46);color:#f3f3f5;pointer-events:auto;animation:login-in .22s cubic-bezier(.2,.9,.24,1)}" +
      "@keyframes login-in{from{opacity:0;transform:translateY(14px) scale(.97)}to{opacity:1;transform:none}}" +
      ".login h3{font-size:13px;font-weight:640;margin-bottom:3px}" +
      ".login p{font-size:11.5px;line-height:1.45;color:#a9a9b6;margin-bottom:12px}" +
      ".login .who{display:flex;align-items:center;gap:9px;padding:9px 10px;border-radius:9px;background:rgba(255,255,255,.06);margin-bottom:12px}" +
      ".login .who i{display:grid;place-items:center;width:26px;height:26px;border-radius:8px;background:rgba(255,255,255,.09);font-style:normal;font-size:12px;font-weight:700}" +
      ".login .who span{font-size:11.5px;line-height:1.35;color:#d8d8e0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}" +
      ".login .acts{display:flex;gap:7px;justify-content:flex-end}" +
      ".login button{border:0;border-radius:8px;padding:7px 12px;font:inherit;font-size:11.5px;cursor:default;transition:filter .12s ease,background .12s ease}" +
      ".login .later{background:rgba(255,255,255,.08);color:#d8d8e0}" +
      ".login .save{background:#6366f1;color:#fff}" +
      ".login button:hover{filter:brightness(1.1)}" +
      ".translator{position:fixed;top:16px;right:16px;width:min(362px,calc(100vw - 32px));padding:14px;border:1px solid rgba(127,127,140,.3);border-radius:16px;background:rgba(24,24,28,.96);backdrop-filter:blur(20px);box-shadow:0 26px 64px rgba(0,0,0,.48);color:#f3f3f5;pointer-events:auto;animation:tr-in .22s cubic-bezier(.2,.9,.24,1)}" +
      "@keyframes tr-in{from{opacity:0;transform:translateY(-10px) scale(.97)}to{opacity:1;transform:none}}" +
      ".trhead{display:flex;align-items:flex-start;gap:10px;margin-bottom:12px}" +
      ".trhead b{display:block;font-size:13px;font-weight:650;letter-spacing:-.01em}" +
      ".trstatus{display:block;margin-top:3px;font-size:11px;line-height:1.4;color:#a9a9b6;min-height:15px}" +
      ".trclose{flex:none;display:grid;place-items:center;width:24px;height:24px;margin:-3px -3px 0 0;padding:0;border:0;border-radius:7px;background:rgba(255,255,255,.07);color:#c9c9d4;font:inherit;font-size:15px;line-height:1;cursor:default}" +
      ".trclose:hover{background:rgba(255,255,255,.14);color:#fff}" +
      ".trrow{display:flex;align-items:flex-end;gap:8px;margin-bottom:11px}" +
      ".trrow label{flex:1;min-width:0;display:flex;flex-direction:column;gap:5px;font-size:10.5px;font-weight:600;letter-spacing:.04em;text-transform:uppercase;color:#9b9ba8}" +
      ".trrow select{width:100%;height:32px;padding:0 26px 0 10px;border:1px solid rgba(127,127,140,.28);border-radius:9px;background:rgba(255,255,255,.06);color:#f3f3f5;font:inherit;font-size:12px;cursor:default;outline:none}" +
      ".trrow select:focus{border-color:#8f8ff9}" +
      ".trrow select option{background:#1c1c20;color:#f3f3f5}" +
      ".trarrow{flex:none;padding-bottom:8px;color:#777785;font-size:14px}" +
      ".tracts{display:flex;gap:7px;justify-content:flex-end}" +
      ".tracts button{border:0;border-radius:9px;padding:8px 13px;font:inherit;font-size:11.5px;font-weight:600;cursor:default;transition:filter .12s ease,background .12s ease}" +
      ".tracts .trrun{background:#6366f1;color:#fff}" +
      ".tracts .trrestore{background:rgba(255,255,255,.08);color:#d8d8e0}" +
      ".tracts button:hover{filter:brightness(1.1)}" +
      ".tracts button:disabled{opacity:.45;filter:none}" +
      "</style>" +
      "<div class='layer'>" +
      "<div class='menu' hidden></div>" +
      "<div class='drop' hidden><div class='dropcard'>Rilascia qui<small>Un'immagine, un file o un indirizzo</small></div></div>" +
      "<div class='preview' hidden><div class='pvbar'><span></span><button type='button'>Chiudi</button></div><img alt=''></div>" +
      "<div class='translator' hidden>" +
      "<div class='trhead'><div><b>Traduci pagina</b><span class='trstatus'>Scegli le lingue e avvia</span></div><button type='button' class='trclose' aria-label='Chiudi'>×</button></div>" +
      "<div class='trrow'><label>Da<select class='trfrom'></select></label><span class='trarrow'>→</span><label>A<select class='trto'></select></label></div>" +
      "<div class='tracts'><button type='button' class='trrestore'>Ripristina</button><button type='button' class='trrun'>Traduci</button></div>" +
      "</div>" +
      "<div class='login' hidden>" +
      "<h3>Salvare la password?</h3>" +
      "<p>Solo se confermi, Deeps la copia nella cassaforte cifrata locale.</p>" +
      "<div class='who'><i></i><span></span></div>" +
      "<div class='acts'><button type='button' class='later'>Non ora</button><button type='button' class='save'>Salva</button></div>" +
      "</div>" +
      "<div class='toast'></div>" +
      "</div>";
    (document.body || document.documentElement).appendChild(host);
    ui = {
      host: host,
      root: root,
      menu: root.querySelector(".menu"),
      drop: root.querySelector(".drop"),
      preview: root.querySelector(".preview"),
      previewImg: root.querySelector(".preview img"),
      previewName: root.querySelector(".pvbar span"),
      previewClose: root.querySelector(".pvbar button"),
      login: root.querySelector(".login"),
      loginWho: root.querySelector(".who span"),
      loginSite: root.querySelector(".who i"),
      loginLater: root.querySelector(".later"),
      loginSave: root.querySelector(".save"),
      translator: root.querySelector(".translator"),
      trStatus: root.querySelector(".trstatus"),
      trFrom: root.querySelector(".trfrom"),
      trTo: root.querySelector(".trto"),
      trRun: root.querySelector(".trrun"),
      trRestore: root.querySelector(".trrestore"),
      trClose: root.querySelector(".trclose"),
      toast: root.querySelector(".toast")
    };
    ui.previewClose.addEventListener("click", closePreview);
    ui.preview.addEventListener("click", function (event) {
      if (event.target === ui.preview) closePreview();
    });
    translatorSetup(ui);
    return ui;
  }

  var toastTimer = 0;
  function toast(message) {
    var box = deepsUi();
    box.toast.textContent = message;
    box.toast.classList.add("show");
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () { box.toast.classList.remove("show"); }, 3200);
  }

  /* ---------------------------------------------------------------- */
  /* live page translator                                             */
  /* ---------------------------------------------------------------- */

  var LANGUAGES = [
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
    ["th", "Thailandese"]
  ];

  var translator = {
    active: false,
    busy: false,
    session: 0,
    seq: 1,
    pending: 0,
    done: 0,
    failed: 0,
    lastError: "",
    jobs: {},
    originals: new Map(),
    translated: new WeakSet(),
    observer: null,
    timer: 0,
    from: window.__deepsTranslateFrom || "auto",
    to: window.__deepsTranslateTo || "it"
  };

  function languageLabel(code) {
    for (var i = 0; i < LANGUAGES.length; i += 1) {
      if (LANGUAGES[i][0] === code) return LANGUAGES[i][1];
    }
    return code;
  }

  function fillLanguages(select, includeAuto) {
    if (!select) return;
    select.textContent = "";
    for (var i = 0; i < LANGUAGES.length; i += 1) {
      if (!includeAuto && LANGUAGES[i][0] === "auto") continue;
      var option = document.createElement("option");
      option.value = LANGUAGES[i][0];
      option.textContent = LANGUAGES[i][1];
      select.appendChild(option);
    }
  }

  function trStatus(message) {
    var box = deepsUi();
    box.trStatus.textContent = message;
  }

  function syncTranslatorSelects() {
    var box = deepsUi();
    var from = window.__deepsTranslateFrom || "auto";
    var to = window.__deepsTranslateTo || "it";
    if (!translator.active) {
      translator.from = from;
      translator.to = to;
    }
    if (box.trFrom) box.trFrom.value = translator.from;
    if (box.trTo) box.trTo.value = translator.to;
  }

  function translatorSetup(box) {
    fillLanguages(box.trFrom, true);
    fillLanguages(box.trTo, false);
    syncTranslatorSelects();
    box.trClose.addEventListener("click", function () { box.translator.hidden = true; });
    box.trRun.addEventListener("click", runTranslator);
    box.trRestore.addEventListener("click", function () { restorePage(false); });
  }

  function runTranslator() {
    var box = deepsUi();
    var from = box.trFrom.value || "auto";
    var to = box.trTo.value || "it";
    window.__deepsTranslateFrom = from;
    window.__deepsTranslateTo = to;
    post({ k: "translate_prefs", f: from, t: to });
    if (translator.active) restorePage(true);
    translatePage(from, to);
  }

  window.__deepsTranslatePopup = function () {
    var box = deepsUi();
    syncTranslatorSelects();
    box.translator.hidden = false;
    if (!translator.active) {
      trStatus("Pronto: " + languageLabel(translator.from) + " → " + languageLabel(translator.to));
    }
    return true;
  };
  window.__deepsTranslateSync = syncTranslatorSelects;

  var SKIP_TAGS = {
    SCRIPT: 1, STYLE: 1, NOSCRIPT: 1, TEXTAREA: 1, INPUT: 1, SELECT: 1, OPTION: 1,
    CODE: 1, PRE: 1, KBD: 1, SAMP: 1, SVG: 1, CANVAS: 1, IFRAME: 1, VIDEO: 1, AUDIO: 1
  };

  function skipTextNode(node) {
    var parent = node.parentElement;
    if (!parent || SKIP_TAGS[parent.tagName]) return true;
    if (parent.closest && parent.closest("[contenteditable=''], [contenteditable='true'], [data-deeps-no-translate]")) return true;
    if (translator.originals.has(node)) return true;
    var text = node.nodeValue;
    if (!text || text.trim().length < 2) return true;
    if (!/[A-Za-z\u00C0-\u024F\u0400-\u04FF\u0590-\u05FF\u0600-\u06FF\u0900-\u097F\u3040-\u30FF\u4E00-\u9FFF\uAC00-\uD7AF]/.test(text)) return true;
    if (!parent.getClientRects || parent.getClientRects().length === 0) return true;
    return false;
  }

  function collectText() {
    var groups = new Map();
    var visited = 0;
    var root = document.body || document.documentElement;
    if (!root) return groups;
    var walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
      acceptNode: function (node) {
        return skipTextNode(node) ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT;
      }
    }, false);
    var node = walker.nextNode();
    while (node) {
      visited += 1;
      if (visited > 700) break;
      var text = node.nodeValue.trim();
      if (text) {
        var list = groups.get(text);
        if (!list) { list = []; groups.set(text, list); }
        list.push(node);
        translator.originals.set(node, node.nodeValue);
      }
      node = walker.nextNode();
    }
    return groups;
  }

  function flushBatch(batch, nodes) {
    if (!batch.length) return;
    var id = translator.seq;
    translator.seq += 1;
    translator.jobs[id] = { texts: batch, nodes: nodes };
    translator.pending += 1;
    post({ k: "page_translate", i: id, f: translator.from, t: translator.to, x: batch });
  }

  function queueGroups(groups) {
    var batch = [];
    var nodes = [];
    var chars = 0;
    groups.forEach(function (list, text) {
      if (text.length > 2000) return;
      if (batch.length >= 20 || (chars + text.length > 1600 && batch.length)) {
        flushBatch(batch, nodes);
        batch = [];
        nodes = [];
        chars = 0;
      }
      batch.push(text);
      nodes.push(list);
      chars += text.length;
    });
    flushBatch(batch, nodes);
    updateTranslatorStatus();
  }

  function updateTranslatorStatus() {
    if (translator.pending > 0) {
      trStatus("Traduzione in corso…" + (translator.done ? " · " + translator.done + " testi" : ""));
      return;
    }
    if (translator.lastError) {
      trStatus("Errore: " + translator.lastError);
      return;
    }
    if (translator.done > 0) {
      trStatus("Pagina tradotta · " + translator.done + " testi");
      return;
    }
    trStatus("Nessun testo da tradurre");
  }

  function translatePage(from, to) {
    translator.from = from;
    translator.to = to;
    translator.session += 1;
    translator.active = true;
    translator.busy = true;
    translator.pending = 0;
    translator.done = 0;
    translator.failed = 0;
    translator.lastError = "";
    translator.jobs = {};
    translator.originals = new Map();
    translator.translated = new WeakSet();
    trStatus("Traduzione in corso…");
    var groups = collectText();
    queueGroups(groups);
    if (!translator.pending) updateTranslatorStatus();
    watchPage();
  }

  function restorePage(keepStatus) {
    translator.session += 1;
    translator.active = false;
    translator.busy = false;
    translator.pending = 0;
    translator.done = 0;
    translator.failed = 0;
    translator.lastError = "";
    translator.jobs = {};
    stopWatching();
    clearTimeout(translator.timer);
    translator.originals.forEach(function (original, node) {
      if (node.isConnected) node.nodeValue = original;
    });
    translator.originals = new Map();
    translator.translated = new WeakSet();
    if (!keepStatus) trStatus("Pagina originale ripristinata");
  }

  function stopWatching() {
    if (translator.observer) {
      translator.observer.disconnect();
      translator.observer = null;
    }
  }

  function watchPage() {
    stopWatching();
    if (!document.body || !window.MutationObserver) return;
    translator.observer = new MutationObserver(function (records) {
      if (!translator.active) return;
      var fresh = false;
      for (var i = 0; i < records.length; i += 1) {
        var record = records[i];
        if (record.type === "characterData") {
          if (!translator.translated.has(record.target)) fresh = true;
        } else if (record.addedNodes && record.addedNodes.length) {
          fresh = true;
        }
        if (fresh) break;
      }
      if (!fresh) return;
      clearTimeout(translator.timer);
      translator.timer = setTimeout(function () {
        if (!translator.active) return;
        var groups = collectText();
        if (groups.size) queueGroups(groups);
      }, 700);
    });
    translator.observer.observe(document.body, {
      subtree: true,
      childList: true,
      characterData: true
    });
  }

  window.__deepsTranslated = function (id, list, error) {
    var job = translator.jobs[id];
    if (!job) return;
    delete translator.jobs[id];
    translator.pending = Math.max(0, translator.pending - 1);
    if (error) {
      translator.failed += 1;
      translator.lastError = String(error);
    } else if (list && list.length === job.texts.length) {
      for (var i = 0; i < list.length; i += 1) {
        var translated = list[i];
        if (!translated) continue;
        var targets = job.nodes[i] || [];
        for (var j = 0; j < targets.length; j += 1) {
          var node = targets[j];
          if (!translator.active || !node.isConnected) continue;
          translator.translated.add(node);
          node.nodeValue = translated;
          translator.done += 1;
        }
      }
    } else {
      translator.failed += 1;
      translator.lastError = "Risposta non valida";
    }
    updateTranslatorStatus();
  };

  /* ---------------------------------------------------------------- */
  /* image drag and drop                                               */
  /* ---------------------------------------------------------------- */

  var dragDepth = 0;
  var previewUrl = "";

  function closePreview() {
    if (!ui) return;
    ui.preview.hidden = true;
    if (previewUrl) { try { URL.revokeObjectURL(previewUrl); } catch (error) {} previewUrl = ""; }
  }

  function showPreview(url, name) {
    var box = deepsUi();
    if (previewUrl) { try { URL.revokeObjectURL(previewUrl); } catch (error) {} }
    previewUrl = url.indexOf("blob:") === 0 ? url : "";
    box.previewImg.src = url;
    box.previewName.textContent = name || "Immagine";
    box.preview.hidden = false;
  }

  function hasUri(data) {
    if (!data) return false;
    var types = data.types || [];
    for (var i = 0; i < types.length; i += 1) {
      if (types[i] === "text/uri-list" || types[i] === "text/plain" || types[i] === "Files") return true;
    }
    return false;
  }

  document.addEventListener("pointerdown", function (event) {
    var target = event.target;
    if (target && target.tagName === "IMG") target.draggable = true;
  }, true);

  document.addEventListener("dragstart", function (event) {
    var target = event.target;
    var image = target && target.closest ? target.closest("img") : null;
    if (!image) return;
    var src = image.currentSrc || image.src;
    if (!src) return;
    try {
      event.dataTransfer.effectAllowed = "copyLink";
      event.dataTransfer.setData("text/uri-list", src);
      event.dataTransfer.setData("text/plain", src);
      event.dataTransfer.setData("text/html", "<img src='" + src.replace(/'/g, "&#39;") + "'>");
    } catch (error) {}
  }, true);

  document.addEventListener("dragenter", function (event) {
    if (!hasUri(event.dataTransfer)) return;
    dragDepth += 1;
    if (dragDepth === 1) deepsUi().drop.hidden = false;
  }, true);
  document.addEventListener("dragover", function (event) {
    if (!hasUri(event.dataTransfer)) return;
    event.preventDefault();
    try { event.dataTransfer.dropEffect = "copy"; } catch (error) {}
  }, true);
  document.addEventListener("dragleave", function () {
    dragDepth = Math.max(0, dragDepth - 1);
    if (dragDepth === 0 && ui) ui.drop.hidden = true;
  }, true);
  document.addEventListener("drop", function (event) {
    dragDepth = 0;
    if (ui) ui.drop.hidden = true;
    var data = event.dataTransfer;
    if (!data) return;
    if (data.files && data.files.length) {
      var file = data.files[0];
      event.preventDefault();
      if (file.type.indexOf("image/") === 0) {
        showPreview(URL.createObjectURL(file), file.name);
      } else {
        toast("Solo le immagini possono essere trascinate qui");
      }
      return;
    }
    var uri = (data.getData("text/uri-list") || data.getData("text/plain") || "").trim();
    if (/^https?:\/\//i.test(uri)) {
      event.preventDefault();
      post({ k: "open_url", u: uri, n: false });
    }
  }, true);

  /* ---------------------------------------------------------------- */
  /* custom context menu                                               */
  /* ---------------------------------------------------------------- */

  var menuOpen = false;

  function closeMenu() {
    if (ui) ui.menu.hidden = true;
    menuOpen = false;
  }

  function copyText(text) {
    var area = document.createElement("textarea");
    area.value = text;
    area.setAttribute("readonly", "");
    area.style.cssText = "position:fixed;left:-9999px;top:-9999px;opacity:0";
    (document.body || document.documentElement).appendChild(area);
    area.select();
    var ok = false;
    try { ok = document.execCommand("copy"); } catch (error) {}
    area.remove();
    try {
      if (navigator.clipboard && navigator.clipboard.writeText) navigator.clipboard.writeText(text).catch(function () {});
    } catch (error) {}
    return ok;
  }

  function short(text) {
    var clean = text.replace(/\s+/g, " ").trim();
    return clean.length > 28 ? clean.slice(0, 28) + "…" : clean;
  }

  function openMenu(x, y, items) {
    var box = deepsUi();
    var menu = box.menu;
    menu.textContent = "";
    items.forEach(function (item) {
      if (item === "-") {
        var line = document.createElement("div");
        line.className = "sep";
        menu.appendChild(line);
        return;
      }
      var button = document.createElement("button");
      button.type = "button";
      if (item.disabled) button.disabled = true;
      var icon = document.createElement("span");
      icon.className = "ico";
      icon.textContent = item.icon || "·";
      var text = document.createElement("span");
      text.className = "txt";
      text.textContent = item.label;
      button.appendChild(icon);
      button.appendChild(text);
      if (item.danger) button.className = "danger";
      button.addEventListener("click", function (event) {
        event.preventDefault();
        event.stopPropagation();
        closeMenu();
        try { item.run(); } catch (error) {}
      });
      menu.appendChild(button);
    });
    menu.hidden = false;
    menu.style.left = "0px";
    menu.style.top = "0px";
    var rect = menu.getBoundingClientRect();
    var left = Math.min(x, window.innerWidth - rect.width - 8);
    var top = Math.min(y, window.innerHeight - rect.height - 8);
    menu.style.left = Math.max(8, left) + "px";
    menu.style.top = Math.max(8, top) + "px";
    menuOpen = true;
  }

  function linkOf(node) {
    var anchor = node && node.closest ? node.closest("a[href]") : null;
    if (!anchor) return "";
    try {
      var url = new URL(anchor.getAttribute("href"), location.href).href;
      return /^https?:/i.test(url) ? url : "";
    } catch (error) { return ""; }
  }

  document.addEventListener("contextmenu", function (event) {
    if (event.shiftKey) return;
    event.preventDefault();
    event.stopPropagation();

    var target = event.target;
    var selection = "";
    try { selection = String(window.getSelection()).trim(); } catch (error) {}
    var link = linkOf(target);
    var image = target && target.closest ? target.closest("img") : null;
    var imageUrl = image ? (image.currentSrc || image.src || "") : "";
    var items = [];

    if (link) {
      items.push({ icon: "↗", label: "Apri link in nuova tab", run: function () { post({ k: "open_url", u: link, n: true }); } });
      items.push({ icon: "⧉", label: "Copia indirizzo link", run: function () { copyText(link); toast("Indirizzo copiato"); } });
      items.push("-");
    }
    if (imageUrl) {
      if (/^https?:/i.test(imageUrl)) {
        items.push({ icon: "↗", label: "Apri immagine in nuova tab", run: function () { post({ k: "open_url", u: imageUrl, n: true }); } });
      }
      items.push({ icon: "⧉", label: "Copia indirizzo immagine", run: function () { copyText(imageUrl); toast("Indirizzo immagine copiato"); } });
      if (/^https?:/i.test(imageUrl)) {
        items.push({
          icon: "↓",
          label: "Salva immagine",
          run: function () {
            var anchor = document.createElement("a");
            anchor.href = imageUrl;
            anchor.download = imageUrl.split("/").pop().split("?")[0] || "immagine";
            anchor.rel = "noopener";
            anchor.click();
          }
        });
      }
      items.push("-");
    }
    if (selection) {
      items.push({ icon: "⧉", label: "Copia", run: function () { copyText(selection); toast("Copiato"); } });
      items.push({ icon: "⌕", label: "Cerca “" + short(selection) + "”", run: function () { post({ k: "search", t: selection }); } });
      items.push({ icon: "⇄", label: "Traduci con Deeps", run: function () { post({ k: "translate", t: selection }); } });
      items.push("-");
    }
    items.push({ icon: "←", label: "Indietro", run: function () { send("back"); } });
    items.push({ icon: "→", label: "Avanti", run: function () { send("forward"); } });
    items.push({ icon: "↻", label: "Ricarica", run: function () { send("reload"); } });
    items.push("-");
    items.push({ icon: "⇄", label: "Traduci questa pagina", run: function () { if (window.__deepsTranslatePopup) window.__deepsTranslatePopup(); } });
    items.push({ icon: "⌘", label: "Copia indirizzo pagina", run: function () { copyText(location.href); toast("Indirizzo copiato"); } });
    items.push({ icon: "≡", label: "Ispeziona", run: function () { send("devtools"); } });

    openMenu(event.clientX, event.clientY, items);
  }, true);

  document.addEventListener("pointerdown", function (event) {
    if (!menuOpen) return;
    var path = event.composedPath ? event.composedPath() : [];
    if (ui && path.indexOf(ui.menu) !== -1) return;
    closeMenu();
  }, true);
  window.addEventListener("scroll", function () { if (menuOpen) closeMenu(); }, true);
  window.addEventListener("resize", function () { if (menuOpen) closeMenu(); }, true);
  window.addEventListener("blur", function () { if (menuOpen) closeMenu(); });
  window.addEventListener("keydown", function (event) {
    if (menuOpen && event.key === "Escape") { event.preventDefault(); closeMenu(); }
  }, true);

  /* ---------------------------------------------------------------- */
  /* login capture                                                     */
  /* ---------------------------------------------------------------- */

  var STASH = "__deeps_login";

  function stash(data) {
    try { sessionStorage.setItem(STASH, JSON.stringify(data)); } catch (error) {}
  }

  function takeStash() {
    try {
      var raw = sessionStorage.getItem(STASH);
      if (!raw) return null;
      sessionStorage.removeItem(STASH);
      var data = JSON.parse(raw);
      if (!data || !data.site || !data.pass) return null;
      return data;
    } catch (error) { return null; }
  }

  function formLogin(form) {
    if (window.__deepsSaveLogins === false) return null;
    var inputs = form.querySelectorAll("input");
    var password = null;
    var username = null;
    for (var i = 0; i < inputs.length; i += 1) {
      var input = inputs[i];
      var type = String(input.type || "text").toLowerCase();
      var name = String(input.name || input.id || "").toLowerCase();
      if (type === "password" && input.value) password = input.value;
      if (!username && input.value) {
        if (type === "email" || /user|email|login|account|nick|nome/.test(name)) username = input.value.trim();
      }
    }
    if (!password) return null;
    return {
      site: location.hostname.replace(/^www\./, ""),
      user: username || "",
      pass: password
    };
  }

  function showLogin(data) {
    if (window.__deepsSaveLogins === false) return;
    var box = deepsUi();
    box.loginSite.textContent = (data.site || "?").charAt(0).toUpperCase();
    box.loginWho.textContent = (data.user ? data.user + " · " : "") + data.site;
    box.login.hidden = false;
    box.loginSave.onclick = function () {
      box.login.hidden = true;
      post({ k: "save_login", s: data.site, u: data.user, p: data.pass });
    };
    box.loginLater.onclick = function () { box.login.hidden = true; };
  }

  window.__deepsLoginResult = function (status) {
    if (status === "ok") toast("Password salvata nella cassaforte");
    else if (status === "locked") toast("Cassaforte bloccata: sbloccala per salvare");
    else if (status === "vault") toast("Crea prima la cassaforte delle password");
    else if (status === "off") toast("Salvataggio password disattivato");
    else toast("Non è stato possibile salvare la password");
  };

  document.addEventListener("submit", function (event) {
    var form = event.target;
    if (!form || form.tagName !== "FORM") return;
    var data = formLogin(form);
    if (!data) return;
    stash(data);
    setTimeout(function () {
      try { sessionStorage.removeItem(STASH); } catch (error) {}
      showLogin(data);
    }, 900);
  }, true);

  window.addEventListener("load", function () {
    var data = takeStash();
    if (data) setTimeout(function () { showLogin(data); }, 420);
  });
})();"##
        .replace("__TOKEN__", token)
        .replace("__INITIAL__", initial)
        .replace("__SAVELOGINS__", save)
        .replace("__VERTICAL__", vertical)
        .replace("__TRANSLATE_FROM__", &from_json)
        .replace("__TRANSLATE_TO__", &to_json)
}
