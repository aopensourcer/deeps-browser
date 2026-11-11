use std::path::Path;

const FALLBACK: &str = r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Deeps</title>
<style>
  html,body{height:100%;margin:0;display:grid;place-items:center;background:#151517;color:#f2f2f3;
  font-family:-apple-system,"Segoe UI",system-ui,sans-serif}
  div{max-width:420px;text-align:center;line-height:1.5}
  code{background:#242428;padding:2px 6px;border-radius:5px;font-size:12px}
  p{color:#9b9ba4;font-size:13px}
</style></head>
<body><div><h2>Chrome UI not built</h2>
<p>Run <code>npm --prefix chrome run build</code> and restart Deeps.</p></div></body></html>"#;

fn main() {
    println!("cargo:rerun-if-changed=chrome/out");

    let out = Path::new("chrome/out");
    if !out.join("index.html").exists() {
        let _ = std::fs::create_dir_all(out);
        let _ = std::fs::write(out.join("index.html"), FALLBACK);
    }
}
