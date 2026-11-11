use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const PAGE: &str = r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>New tab</title>
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<style>
  :root{
    color-scheme:light dark;
    --bg:light-dark(#f4f4f5,#0d0d0f);
    --text:light-dark(#18181b,#f2f2f3);
    --muted:light-dark(#6f6f78,#8f8f99);
    --line:light-dark(rgba(0,0,0,.10),rgba(255,255,255,.09));
    --field:light-dark(#ffffff,#17171a);
    --hover:light-dark(rgba(0,0,0,.045),rgba(255,255,255,.07));
    --accent:light-dark(#4f46e5,#8f8ff9);
  }
  *{box-sizing:border-box}
  html,body{height:100%;margin:0}
  body{
    background:var(--bg);color:var(--text);overflow:hidden;
    font-family:-apple-system,"Segoe UI Variable Display","Segoe UI",system-ui,sans-serif;
    display:grid;place-items:center;
  }
  main{width:min(560px,calc(100vw - 48px));transform:translateY(-3vh);text-align:center}
  .mark{font-size:42px;font-weight:640;letter-spacing:-.045em;margin-bottom:26px}
  .mark em{font-style:normal;color:var(--accent)}
  form{position:relative;display:flex;align-items:center}
  form svg{position:absolute;left:15px;color:var(--muted);pointer-events:none}
  input{
    width:100%;height:48px;padding:0 60px 0 43px;
    border:1px solid var(--line);border-radius:12px;background:var(--field);color:var(--text);
    font:inherit;font-size:14.5px;outline:none;box-shadow:0 1px 2px rgba(0,0,0,.04);
    transition:border-color .15s ease,box-shadow .15s ease;
  }
  input::placeholder{color:var(--muted)}
  input:focus{border-color:var(--accent);box-shadow:0 0 0 4px color-mix(in srgb,var(--accent) 14%,transparent)}
  kbd{
    position:absolute;right:13px;min-width:20px;height:20px;display:grid;place-items:center;
    border:1px solid var(--line);border-radius:5px;color:var(--muted);font:inherit;font-size:11px;background:var(--bg);
  }
  nav{display:flex;flex-wrap:wrap;justify-content:center;gap:8px;margin-top:22px}
  nav a{
    display:flex;align-items:center;gap:7px;height:32px;padding:0 13px;
    border:1px solid var(--line);border-radius:8px;color:var(--muted);text-decoration:none;font-size:12.5px;
    transition:background .12s ease,color .12s ease,border-color .12s ease;
  }
  nav a:hover{background:var(--hover);color:var(--text);border-color:transparent}
  nav i{width:6px;height:6px;border-radius:2px;background:var(--accent);opacity:.7}
  .hint{position:fixed;left:0;right:0;bottom:18px;text-align:center;color:var(--muted);font-size:11.5px;opacity:.7}
</style>
</head>
<body>
  <main>
    <div class="mark">deepy<em>.</em></div>
    <form action="https://duckduckgo.com/" method="get" autocomplete="off">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.2-3.2"/></svg>
      <input name="q" placeholder="Search the web" autocomplete="off" spellcheck="false" autofocus aria-label="Search">
      <kbd>/</kbd>
    </form>
    <nav>
      <a href="https://github.com"><i style="background:#6e7681"></i>GitHub</a>
      <a href="https://news.ycombinator.com"><i style="background:#ff6600"></i>Hacker News</a>
      <a href="https://developer.mozilla.org"><i style="background:#2563eb"></i>MDN</a>
      <a href="https://en.wikipedia.org"><i style="background:#6b7280"></i>Wikipedia</a>
      <a href="https://www.rust-lang.org"><i style="background:#c2410c"></i>Rust</a>
    </nav>
  </main>
  <div class="hint">Ctrl+T new tab · Ctrl+L address bar</div>
  <script>
    var input = document.querySelector("input");
    addEventListener("keydown", function (event) {
      if (event.key === "/" && document.activeElement !== input) { event.preventDefault(); input.focus(); }
      if (event.key === "Escape") input.blur();
    });
  </script>
</body>
</html>
"#;

const ICON: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect width="32" height="32" rx="8" fill="#4f46e5"/>
<path d="M11 8.5h4.6a7.5 7.5 0 0 1 0 15H11z" fill="#fff"/>
</svg>"##;

/// Serves the internal new-tab page on an ephemeral loopback port. Using a real
/// origin keeps links, forms and the address bar behaving like a normal page.
pub fn serve() -> std::io::Result<u16> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();

    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            thread::spawn(move || {
                let _ = handle(stream);
            });
        }
    });

    Ok(port)
}

fn handle(mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = [0u8; 4096];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request.split_whitespace().nth(1).unwrap_or("/");

    let (body, content_type) = if target.starts_with("/favicon.svg") {
        (ICON.as_bytes(), "image/svg+xml")
    } else if target.starts_with("/favicon.ico") {
        (b"".as_slice(), "image/x-icon")
    } else {
        (PAGE.as_bytes(), "text/html; charset=utf-8")
    };

    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    Ok(())
}
