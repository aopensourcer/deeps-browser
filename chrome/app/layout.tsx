import type { Metadata, Viewport } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Deeps",
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  maximumScale: 1,
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="it">
      <head>
        <style
          dangerouslySetInnerHTML={{
            __html:
              "html,body{margin:0;background:#0d0d0f}@media (prefers-color-scheme:light){html,body{background:#f6f6f7}}",
          }}
        />
      </head>
      <body>{children}</body>
    </html>
  );
}
